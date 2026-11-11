/* ck_oracle: the differential oracle of the Rust script runtime (docs/drahtseilakt/scripts.md, "Stage 0"). Test-only:
   it builds a graph of trivial blocks from a small text description in the upstream C runtime (src/ck, src/bb, kept
   as a reference), runs it frame by frame and prints the trace the Rust side prints for the same description
   (crates/script/tests/oracle.rs). Nothing of this goes into the game.

   Description, one item a line (indices count from 0 in the order of their kind):
     object NAME                      a behavioural object (manager order = this order)
     script NAME OBJECT ACTIVE        a script on an object, active (1) or deactivated (0) at the start
     block NAME SCRIPT KIND PRIO [A [B]]  a building block in a script; KIND nop (1 in, 1 out), switch A (1 in,
                                      2 outs, condition A), activate A B (Activate Script of script A, Reset? B),
                                      deactivate A (Deactivate Script of script A)
     bb NAME SCRIPT GA:GB PRIO NIN NOUT  level 2: a block of any prototype GUID with NIN inputs, NOUT outputs
     pin|pout|local BLOCK TYPE VALUE  a parameter of that block, in order: TYPE bool, int, float, time (float ms),
                                      string, msg (a message type by name), obj / script / group (an index of
                                      that kind, -1 none), none (a pin with no source: the block's default),
                                      vector / vec2 / box (comma-separated floats), percentage (a float)
     group NAME OBJECT...             a group of objects
     array NAME COLS ROWS KEY         an array of Integer columns, cell (row, col) = 10 * row + col, key column
                                      KEY (-1 none)
     target BLOCK TYPE VALUE          the block's target parameter (TYPE as for pin; array: an array index)
     link SCRIPT FROM TO DELAY        FROM: s (the script's start) or BLOCK.OUT; TO: BLOCK (its input 0) or
                                      BLOCK.IN
     frames N
   Output: "frame N", then "run NAME" for each block run, then "end" and the names of the active blocks and
   scripts, then "pout BLOCK K VALUE" for each output parameter of a level-2 block (floats as their bits in hex,
   integers in decimal, objects by name, "-" none). */
#include "ck/ck.h"
#include "ck/ck_types.h"
#include "bb/bb.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAXN 4096

static CkContext ctx;
static CkId objects[MAXN], scripts[MAXN], blocks[MAXN], groups[MAXN], arrays[MAXN];
static unsigned nobjects, nscripts, nblocks, ngroups, narrays;
/* the level-2 blocks' output parameters, with the type they are printed by */
static struct { unsigned block, index; CkId param; char type[16]; } pouts[MAXN];
static unsigned npouts;

static void trace(CkContext *c, CkBehavior *b, const char *what)
{
    (void)c;
    if (!strcmp(what, "execute")) printf("run %s\n", b->h.name);
}

static CkBehaviorIO *new_io(CkBehavior *owner, bool input)
{
    CkBehaviorIO *io = (CkBehaviorIO *)ck_create(&ctx, CKCID_BEHAVIORIO, input ? "In" : "Out");
    io->input = input;
    io->owner = owner->h.id;
    ck_ids_push(input ? &owner->in : &owner->out, io->h.id);
    return io;
}

/* an input parameter of b reading a local of the given type and 4-byte value */
static void new_pin(CkBehavior *b, CkGuid type, uint32_t value)
{
    CkParameter *src = (CkParameter *)ck_create(&ctx, CKCID_PARAMETERLOCAL, "value");
    src->kind = CKP_LOCAL;
    src->type = type;
    src->value = malloc(4);
    memcpy(src->value, &value, 4);
    src->size = 4;
    CkParameter *pin = (CkParameter *)ck_create(&ctx, CKCID_PARAMETERIN, "pin");
    pin->kind = CKP_IN;
    pin->type = type;
    pin->source = src->h.id;
    pin->owner = b->h.id;
    ck_ids_push(&b->pin, pin->h.id);
}

static bool type_guid(const char *t, CkGuid *g)
{
    static const struct { const char *name; CkGuid g; } k[] = {
        {"bool", {0x1ad52a8eu, 0x5e741920u}},   {"int", {0x5a5716fdu, 0x44e276d7u}},
        {"float", {0x47884c3fu, 0x432c2c20u}},  {"time", {0x54b4422bu, 0x730f0f4fu}},
        {"string", {0x6bd010e2u, 0x115617eau}}, {"msg", {0x03881e12u, 0x5ba34e2bu}},
        {"obj", {0x71d80779u, 0x402f42f3u}},    {"script", {0x7ea4176du, 0x1b405d30u}},
        {"group", {0x5c0f151bu, 0x6f0547fdu}},  {"none", {0x5a5716fdu, 0x44e276d7u}},
        {"array", {0x024d52f1u, 0x678223b2u}},  {"vector", {0x48824eaeu, 0x2fe47960u}},
        {"vec2", {0x4efcb34au, 0x6079e42fu}},   {"box", {0x668649c8u, 0x283e2ee1u}},
        {"percentage", {0xf3c84b4eu, 0x0ffacc34u}}, {"classid", {0x19644e43u, 0x4d6f6123u}},
    };
    for (size_t i = 0; i < sizeof k / sizeof *k; i++)
        if (!strcmp(t, k[i].name)) {
            *g = k[i].g;
            return true;
        }
    return false;
}

/* the float components of a vector-like type (vector 3, vec2 2, box 6), 0 for a 4-byte type */
static unsigned components(const char *t)
{
    return !strcmp(t, "vector") ? 3 : !strcmp(t, "vec2") ? 2 : !strcmp(t, "box") ? 6 : 0;
}

/* a parameter's value from its description: 4 bytes, a string with its terminator, or comma-separated floats */
static void set_value(CkParameter *p, const char *t, const char *v)
{
    uint32_t w = 0;
    unsigned n = components(t);
    if (n) {
        float *f = calloc(n, sizeof *f);
        const char *q = v;
        for (unsigned k = 0; k < n && q && *q; k++) {
            f[k] = strtof(q, NULL);
            q = strchr(q, ',');
            if (q) q++;
        }
        p->value = (uint8_t *)f;
        p->size = n * (uint32_t)sizeof *f;
        return;
    }
    if (!strcmp(t, "string")) {
        size_t n = strlen(v) + 1;
        p->value = malloc(n);
        memcpy(p->value, v, n);
        p->size = (uint32_t)n;
        return;
    }
    if (!strcmp(t, "float") || !strcmp(t, "time") || !strcmp(t, "percentage")) {
        float f = strtof(v, NULL);
        memcpy(&w, &f, 4);
    } else if (!strcmp(t, "msg")) {
        int32_t m = ck_message_type(&ctx, v);
        memcpy(&w, &m, 4);
    } else if (!strcmp(t, "obj") || !strcmp(t, "script") || !strcmp(t, "group") || !strcmp(t, "array")) {
        long i = strtol(v, NULL, 10);
        CkId *of = !strcmp(t, "obj") ? objects : !strcmp(t, "script") ? scripts : !strcmp(t, "group") ? groups : arrays;
        unsigned n = !strcmp(t, "obj") ? nobjects : !strcmp(t, "script") ? nscripts : !strcmp(t, "group") ? ngroups : narrays;
        w = i >= 0 && (unsigned long)i < n ? of[i] : 0;
    } else {
        int32_t i = (int32_t)strtol(v, NULL, 10);
        memcpy(&w, &i, 4);
    }
    p->value = malloc(4);
    memcpy(p->value, &w, 4);
    p->size = 4;
}

static void print_value(const CkParameter *p, const char *t)
{
    uint32_t w = 0;
    if (p->value) memcpy(&w, p->value, p->size < 4 ? p->size : 4);
    unsigned n = components(t);
    if (n) {
        for (unsigned k = 0; k < n; k++) {
            uint32_t c = 0;
            if (p->value && (k + 1) * 4 <= p->size) memcpy(&c, (const uint8_t *)p->value + 4 * k, 4);
            printf("%s%08x", k ? "," : "", c);
        }
    } else if (!strcmp(t, "string")) {
        printf("%s", p->value && p->size ? (const char *)p->value : "");
    } else if (!strcmp(t, "float") || !strcmp(t, "time") || !strcmp(t, "percentage")) {
        printf("%08x", w);
    } else if (!strcmp(t, "obj") || !strcmp(t, "script") || !strcmp(t, "group") || !strcmp(t, "array")) {
        CkObj *o = w ? ck_obj(&ctx, w) : NULL;
        printf("%s", o ? o->name : "-");
    } else {
        printf("%d", (int32_t)w);
    }
}

static const CkBBDecl *find_bb(CkGuid g)
{
    for (uint32_t k = 0; k < ctx.nbbs; k++)
        if (ck_guid_eq(ctx.bbs[k]->guid, g)) return ctx.bbs[k];
    return NULL;
}

int main(int argc, char **argv)
{
    FILE *f = argc > 1 ? fopen(argv[1], "r") : stdin;
    if (!f) return 2;
    ck_init(&ctx);
    bb_register_all(&ctx);
    ctx.trace = trace;
    char line[512];
    int frames = 0;
    while (fgets(line, sizeof line, f)) {
        char kind[32] = "", name[128] = "", k2[32] = "", from[32] = "";
        unsigned a = 0, b = 0, c = 0;
        int prio = 0;
        if (sscanf(line, "%31s", kind) != 1) continue;
        if (!strcmp(kind, "object") && sscanf(line, "%*s %127s", name) == 1 && nobjects < MAXN) {
            CkObj *o = ck_create(&ctx, CKCID_BEOBJECT, name);
            objects[nobjects++] = o->id;
            ck_ids_push(&ctx.managed, o->id);
        } else if (!strcmp(kind, "script") && sscanf(line, "%*s %127s %u %u", name, &a, &b) == 3 && a < nobjects &&
                   nscripts < MAXN) {
            CkBehavior *s = (CkBehavior *)ck_create(&ctx, CKCID_BEHAVIOR, name);
            s->bflags = CKBF_SCRIPT;
            s->owner = objects[a];
            new_io(s, true);
            ck_ids_push(&ck_beobject(&ctx, objects[a])->scripts, s->h.id);
            scripts[nscripts++] = s->h.id;
            ck_behavior_activate(&ctx, s, b != 0, true);
        } else if (!strcmp(kind, "block") &&
                   sscanf(line, "%*s %127s %u %31s %d %u %u", name, &a, k2, &prio, &b, &c) >= 4 && a < nscripts &&
                   nblocks < MAXN) {
            CkBehavior *s = ck_behavior(&ctx, scripts[a]);
            CkBehavior *x = (CkBehavior *)ck_create(&ctx, CKCID_BEHAVIOR, name);
            x->has_proto = true;
            x->bflags = CKBF_BUILDINGBLOCK;
            x->priority = prio;
            x->parent = s->h.id;
            x->owner = s->owner;
            new_io(x, true);
            new_io(x, false);
            if (!strcmp(k2, "nop")) {
                x->proto = (CkGuid){0x302561c4u, 0x0d282980u};
            } else if (!strcmp(k2, "switch")) {
                x->proto = (CkGuid){0xeb506901u, 0x984afcccu};
                new_io(x, false);
                new_pin(x, CKPGUID_BOOL, b);
            } else if (!strcmp(k2, "activate") && b < nscripts) {
                x->proto = (CkGuid){0x4c7e7bc3u, 0x0b693155u};
                new_pin(x, CKPGUID_BOOL, c);
                new_pin(x, (CkGuid){0x7ea4176du, 0x1b405d30u}, scripts[b]);   /* Script */
            } else if (!strcmp(k2, "deactivate") && b < nscripts) {
                x->proto = (CkGuid){0x14367c05u, 0x635b24f9u};
                new_pin(x, (CkGuid){0x7ea4176du, 0x1b405d30u}, scripts[b]);   /* Script */
            } else {
                fprintf(stderr, "bad block: %s", line);
                return 2;
            }
            x->bb = find_bb(x->proto);
            ck_ids_push(&s->sub, x->h.id);
            blocks[nblocks++] = x->h.id;
        } else if (!strcmp(kind, "link") && sscanf(line, "%*s %u %31s %31s %u", &a, from, k2, &c) == 4 && a < nscripts &&
                   sscanf(k2, "%u", &b) == 1 && b < nblocks) {
            unsigned in = 0;
            if (strchr(k2, '.')) sscanf(strchr(k2, '.') + 1, "%u", &in);
            CkBehavior *s = ck_behavior(&ctx, scripts[a]);
            CkBehaviorIO *src;
            if (!strcmp(from, "s")) {
                src = ck_io(&ctx, s->in.v[0]);
            } else {
                unsigned blk = 0, out = 0;
                if (sscanf(from, "%u.%u", &blk, &out) != 2 || blk >= nblocks) return 2;
                CkBehavior *x = ck_behavior(&ctx, blocks[blk]);
                if (out >= x->out.n) return 2;
                src = ck_io(&ctx, x->out.v[out]);
            }
            CkBehavior *dst = ck_behavior(&ctx, blocks[b]);
            CkBehaviorLink *l = (CkBehaviorLink *)ck_create(&ctx, CKCID_BEHAVIORLINK, "link");
            l->src = src->h.id;
            if (in >= dst->in.n) return 2;
            l->dst = dst->in.v[in];
            l->init_delay = l->cur_delay = (int16_t)c;
            ck_ids_push(&src->links, l->h.id);
            ck_ids_push(&s->links, l->h.id);
        } else if (!strcmp(kind, "group") && ngroups < MAXN) {
            int at = 0;
            if (sscanf(line, "%*s %127s %n", name, &at) < 1) return 2;
            CkGroup *g = (CkGroup *)ck_create(&ctx, CKCID_GROUP, name);
            const char *q = line + at;
            unsigned m;
            int used;
            while (sscanf(q, "%u%n", &m, &used) == 1) {
                if (m >= nobjects) return 2;
                ck_ids_push(&g->members, objects[m]);
                q += used;
            }
            groups[ngroups++] = g->be.h.id;
        } else if (!strcmp(kind, "array") && narrays < MAXN) {
            unsigned cols = 0, rows = 0;
            int key = -1;
            if (sscanf(line, "%*s %127s %u %u %d", name, &cols, &rows, &key) != 4 || cols > 16 || rows > 64) return 2;
            CkDataArray *arr = (CkDataArray *)ck_create(&ctx, CKCID_DATAARRAY, name);
            for (unsigned k = 0; k < cols; k++) ck_array_insert_column(&ctx, arr, -1, CKARRAYTYPE_INT, "c", CKPGUID_NONE);
            for (unsigned r = 0; r < rows; r++) {
                uint32_t row = ck_array_add_row(&ctx, arr);
                for (unsigned k = 0; k < cols; k++) {
                    CkCell *c = ck_array_cell(arr, row, k);
                    if (c) c->i = (int32_t)(10 * r + k);
                }
            }
            arr->key_column = key;
            arrays[narrays++] = arr->be.h.id;
        } else if (!strcmp(kind, "target") && sscanf(line, "%*s %u %31s %127s", &a, k2, name) == 3 && a < nblocks) {
            CkBehavior *x = ck_behavior(&ctx, blocks[a]);
            CkGuid g;
            if (!type_guid(k2, &g)) return 2;
            CkParameter *src = (CkParameter *)ck_create(&ctx, CKCID_PARAMETERLOCAL, "value");
            src->kind = CKP_LOCAL;
            src->type = g;
            set_value(src, k2, name);
            CkParameter *pin = (CkParameter *)ck_create(&ctx, CKCID_PARAMETERIN, "target");
            pin->kind = CKP_IN;
            pin->type = g;
            pin->owner = x->h.id;
            pin->source = src->h.id;
            x->target = pin->h.id;
        } else if (!strcmp(kind, "bb") && nblocks < MAXN) {
            unsigned ga = 0, gb = 0, nin = 0, nout = 0;
            if (sscanf(line, "%*s %127s %u %x:%x %d %u %u", name, &a, &ga, &gb, &prio, &nin, &nout) != 7 ||
                a >= nscripts || nin > 32 || nout > 32)
                return 2;
            CkBehavior *s = ck_behavior(&ctx, scripts[a]);
            CkBehavior *x = (CkBehavior *)ck_create(&ctx, CKCID_BEHAVIOR, name);
            x->has_proto = true;
            x->bflags = CKBF_BUILDINGBLOCK;
            x->priority = prio;
            x->parent = s->h.id;
            x->owner = s->owner;
            x->proto = (CkGuid){ga, gb};
            x->proto_version = 0x20000;
            for (unsigned k = 0; k < nin; k++) new_io(x, true);
            for (unsigned k = 0; k < nout; k++) new_io(x, false);
            x->bb = find_bb(x->proto);
            if (!x->bb) {
                fprintf(stderr, "no block %08x:%08x\n", ga, gb);
                return 3;
            }
            ck_ids_push(&s->sub, x->h.id);
            blocks[nblocks++] = x->h.id;
        } else if ((!strcmp(kind, "pin") || !strcmp(kind, "pout") || !strcmp(kind, "local")) &&
                   sscanf(line, "%*s %u %31s %127s", &a, k2, name) >= 2 && a < nblocks) {
            CkBehavior *x = ck_behavior(&ctx, blocks[a]);
            CkGuid g;
            if (!type_guid(k2, &g)) return 2;
            if (!strcmp(kind, "pin")) {
                CkParameter *pin = (CkParameter *)ck_create(&ctx, CKCID_PARAMETERIN, "pin");
                pin->kind = CKP_IN;
                pin->type = g;
                pin->owner = x->h.id;
                if (strcmp(k2, "none")) {
                    CkParameter *src = (CkParameter *)ck_create(&ctx, CKCID_PARAMETERLOCAL, "value");
                    src->kind = CKP_LOCAL;
                    src->type = g;
                    set_value(src, k2, name);
                    pin->source = src->h.id;
                }
                ck_ids_push(&x->pin, pin->h.id);
            } else {
                bool out = !strcmp(kind, "pout");
                CkParameter *p = (CkParameter *)ck_create(&ctx, out ? CKCID_PARAMETEROUT : CKCID_PARAMETERLOCAL,
                                                          out ? "pout" : "local");
                p->kind = out ? CKP_OUT : CKP_LOCAL;
                p->type = g;
                p->owner = x->h.id;
                set_value(p, k2, name);
                ck_ids_push(out ? &x->pout : &x->local, p->h.id);
                if (out && npouts < MAXN) {
                    pouts[npouts].block = a;
                    pouts[npouts].index = x->pout.n - 1;
                    pouts[npouts].param = p->h.id;
                    snprintf(pouts[npouts].type, sizeof pouts[npouts].type, "%s", k2);
                    npouts++;
                }
            }
        } else if (!strcmp(kind, "frames")) {
            sscanf(line, "%*s %d", &frames);
        }
    }
    /* the loader tells every block it was loaded (CKM_BEHAVIORLOAD), as the game's does */
    for (unsigned k = 0; k < nblocks; k++) {
        CkBehavior *x = ck_behavior(&ctx, blocks[k]);
        if (x && x->bb && x->bb->callback) x->bb->callback(&ctx, x, CKM_BEHAVIORLOAD);
    }
    for (int i = 1; i <= frames; i++) {
        printf("frame %d\n", i);
        ck_process(&ctx, 1000.0f / 60.0f);
        printf("end");
        for (unsigned k = 0; k < nscripts; k++) {
            CkBehavior *s = ck_behavior(&ctx, scripts[k]);
            if (s->bflags & CKBF_ACTIVE) printf(" %s", s->h.name);
        }
        for (unsigned k = 0; k < nblocks; k++) {
            CkBehavior *x = ck_behavior(&ctx, blocks[k]);
            if (x->bflags & CKBF_ACTIVE) printf(" %s", x->h.name);
        }
        printf("\n");
        for (unsigned k = 0; k < npouts; k++) {
            printf("pout %s %u ", ck_behavior(&ctx, blocks[pouts[k].block])->h.name, pouts[k].index);
            print_value(ck_param(&ctx, pouts[k].param), pouts[k].type);
            printf("\n");
        }
    }
    return 0;
}
