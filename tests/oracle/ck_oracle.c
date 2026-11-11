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
     link SCRIPT FROM TO DELAY        FROM: s (the script's start) or BLOCK.OUT; TO: BLOCK (its input 0)
     frames N
   Output: "frame N", then "run NAME" for each block run, then "end" and the names of the active blocks and
   scripts. */
#include "ck/ck.h"
#include "ck/ck_types.h"
#include "bb/bb.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define MAXN 4096

static CkContext ctx;
static CkId objects[MAXN], scripts[MAXN], blocks[MAXN];
static unsigned nobjects, nscripts, nblocks;

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
        } else if (!strcmp(kind, "link") && sscanf(line, "%*s %u %31s %u %u", &a, from, &b, &c) == 4 && a < nscripts &&
                   b < nblocks) {
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
            l->dst = dst->in.v[0];
            l->init_delay = l->cur_delay = (int16_t)c;
            ck_ids_push(&src->links, l->h.id);
            ck_ids_push(&s->links, l->h.id);
        } else if (!strcmp(kind, "frames")) {
            sscanf(line, "%*s %d", &frames);
        }
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
    }
    return 0;
}
