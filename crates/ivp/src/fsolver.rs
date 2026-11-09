// Port of OpenBallance src/phys/phys_fsolver.c (GPL-3.0; see the LICENSE of the repository): the linear constraint solver
// of the normal force of a friction system (docs/ivp_contact.md 5, "LS"). A linear complementarity problem: find x >= 0
// with A x + b >= 0 and x (A x + b) = 0, by an active-set walk with the upstream's pivots, permutations and epsilons.
//
// The incremental LU of the upstream (IVP_Incr_L_U_Matrix) is replaced by fresh partial-pivot eliminations of the active
// sub-system, as the upstream's own port note allows (LS4.5): its validity test, the decisions of solve_lc, the
// permutations, the epsilons and the step limit are the upstream's. This file holds the solver and the core reaction of
// the tangential friction (LS5); the friction system that feeds it is in `crate::friction`.

/// The solver's epsilons (ivp_const.h, LS0): TEST_EPS and the maximum error of a full test.
pub const TEST_EPS: f64 = 1.000_000_011_686_097_4e-7;
const GAUSS_EPS: f64 = TEST_EPS;
const MAX_ERROR: f64 = TEST_EPS * 1000.0;
const MAX_STEP: f64 = 10000.0;
const LU_EPS: f64 = TEST_EPS * 10.0;
/// At most this many steps of the active-set walk (IVP_C_MAX_LCP_STEPS).
pub const MAX_LCP_STEPS: u32 = 250;

/// Gauss elimination with partial pivoting of an n x n row-major system, A and b overwritten (LS3.2). Returns 1 when
/// solved; 0 when a zero pivot meets a non-zero right-hand side (x is then cleared).
pub fn gauss_solve(n: usize, a: &mut [f64], b: &mut [f64], x: &mut [f64], eps: f64) -> i32 {
    for p in 0..n {
        // the largest pivot below p (0x10033e80): ties keep the lower row
        let mut best: Option<usize> = None;
        let mut m = a[p * n + p].abs();
        for r in (p + 1..n).rev() {
            if a[r * n + p].abs() > m {
                m = a[r * n + p].abs();
                best = Some(r);
            }
        }
        if let Some(best) = best {
            for c in 0..n {
                a.swap(p * n + c, best * n + c);
            }
            b.swap(p, best);
        }
        let d = a[p * n + p];
        if d.abs() < eps {
            continue;
        }
        let inv = -1.0 / d;
        for r in p + 1..n {
            let f = a[r * n + p];
            if f.abs() > eps {
                let s = f * inv;
                for c in p..n {
                    a[r * n + c] += s * a[p * n + c];
                }
                b[r] += s * b[p];
            }
        }
    }
    for i in (0..n).rev() {
        let mut s = b[i];
        for c in i + 1..n {
            s -= a[i * n + c] * b[c];
        }
        let d = a[i * n + i];
        if d.abs() < eps {
            if s.abs() < eps * 1000.0 {
                b[i] = 0.0;
                continue;
            }
            for v in x.iter_mut().take(n) {
                *v = 0.0;
            }
            return 0;
        }
        b[i] = s / d;
    }
    x[..n].copy_from_slice(&b[..n]);
    1
}

/// The decomposition test of the LU (0x10035ae0): partial pivoting, and a pivot below the LU epsilon fails.
fn lu_decompose_ok(n: usize, u: &mut [f64]) -> bool {
    for p in 0..n {
        let mut best = p;
        let mut m = u[p * n + p].abs();
        for r in (p + 1..n).rev() {
            if u[r * n + p].abs() > m {
                m = u[r * n + p].abs();
                best = r;
            }
        }
        if best != p {
            for c in 0..n {
                u.swap(p * n + c, best * n + c);
            }
        }
        let d = u[p * n + p];
        if d.abs() < LU_EPS {
            return false;
        }
        for r in p + 1..n {
            let f = u[r * n + p] / d;
            if f != 0.0 {
                for c in p..n {
                    u[r * n + c] -= f * u[p * n + c];
                }
            }
        }
    }
    true
}

/// The state of one LCP solve (`IVP_Linear_Constraint_Solver`, LS4): the matrix and right-hand side, the solution and
/// acceleration vectors, the active-set index arrays (`aii` the position of each variable, `found` its inverse), the
/// counts of active and ignored variables, and the permutation counters.
struct Lcs<'a> {
    n: usize,
    a: &'a [f64],
    b: &'a [f64],
    x: Vec<f64>,
    accel: Vec<f64>,
    delta_f: Vec<f64>,
    delta_accel: Vec<f64>,
    reset_x: Vec<f64>,
    reset_accel: Vec<f64>,
    aii: Vec<usize>,
    found: Vec<usize>,
    r_actives: usize,
    ignored_pos: usize,
    p1: usize,
    p2: usize,
    q1: usize,
    q2: usize,
    /// 0: the LU is valid; 1: invalid (Gauss); 2: failed.
    status: i32,
    sub: Vec<f64>,
    sb: Vec<f64>,
    sx: Vec<f64>,
}

impl<'a> Lcs<'a> {
    fn af(&self, r: usize, c: usize) -> f64 {
        self.a[r * self.n + c]
    }

    /// Exchanges two positions of the index array (0x10034f10).
    fn swap_pos(&mut self, p: usize, q: usize) {
        let (a, b) = (self.aii[p], self.aii[q]);
        self.aii[p] = b;
        self.aii[q] = a;
        self.found[b] = p;
        self.found[a] = q;
    }

    /// The active sub-system, m x m, into `sub`.
    fn active_matrix(&mut self, m: usize) {
        for r in 0..m {
            for c in 0..m {
                self.sub[r * m + c] = self.af(self.aii[r], self.aii[c]);
            }
        }
    }

    /// The decomposition of the active sub-system (0x100351c0).
    fn setup_lu(&mut self) -> bool {
        let m = self.r_actives;
        self.active_matrix(m);
        lu_decompose_ok(m, &mut self.sub)
    }

    /// Solves the active sub-system for `input`, into `out` (0x10035ff0). Returns the Gauss code.
    fn lu_solve(&mut self, input: &[f64], out: &mut [f64]) -> i32 {
        let m = self.r_actives;
        self.active_matrix(m);
        self.sb[..m].copy_from_slice(&input[..m]);
        gauss_solve(m, &mut self.sub, &mut self.sb, out, LU_EPS)
    }

    /// The decomposition after a variable left the active set (0x10034870).
    fn lu_remove(&mut self) {
        if self.status == 0 && !self.setup_lu() {
            self.status = 2;
        }
    }

    /// The decomposition after a variable joined the active set (0x100348b0).
    fn lu_add(&mut self) {
        if self.status == 0 && !self.setup_lu() {
            self.status = 2;
        }
    }

    /// The accelerations and solution from the reset solution (0x100352c0).
    fn recompute_x_accel(&mut self) {
        let n = self.n;
        for r in 0..n {
            let mut s = 0.0;
            for c in 0..n {
                s += self.af(r, c) * self.reset_x[c];
            }
            self.reset_accel[r] = s;
        }
        for i in 0..n {
            self.reset_accel[i] -= self.b[i];
        }
        for m in 0..self.r_actives {
            let v = self.aii[m];
            self.reset_accel[v] = 0.0;
        }
        for i in (0..n).rev() {
            self.accel[i] = self.reset_accel[i];
            self.x[i] = self.reset_x[i];
        }
    }

    /// A little deterministic permutation of the active and the ignored variables (0x10034990).
    fn little_random_permutation(&mut self) {
        if self.r_actives < 2 {
            return;
        }
        self.p1 = (self.p1 + 1) % self.r_actives;
        self.p2 = (self.p2 + 2) % self.r_actives;
        self.swap_pos(self.p1, self.p2);
        self.q2 += 2;
        self.q1 += 1;
        let m = self.n as i64 - self.ignored_pos as i64 - 1;
        if m >= 2 {
            let m = m as usize;
            self.q1 %= m;
            self.q2 %= m;
            self.swap_pos(
                self.q1 + 1 + self.ignored_pos,
                self.q2 + 1 + self.ignored_pos,
            );
        }
    }

    /// Moves position `p` to the end of the index array (0x10035680).
    fn rotate_to_end(&mut self, p: usize) {
        for q in p + 1..self.n {
            self.swap_pos(q - 1, q);
        }
    }

    /// Drops the active variables that went negative and the ignored ones with negative acceleration (0x100356b0).
    fn remove_negatives(&mut self) -> u32 {
        let mut count = 0;
        let mut m = 0usize;
        while m < self.r_actives {
            let v = self.aii[m];
            if !(self.x[v] < TEST_EPS) {
                m += 1;
                continue;
            }
            if self.x[v] <= -TEST_EPS {
                self.rotate_to_end(m);
                count += 1;
                self.ignored_pos -= 1;
            } else {
                self.x[v] = 0.0;
                let last = self.r_actives - 1;
                self.swap_pos(m, last);
            }
            self.r_actives -= 1;
            self.status = 1;
            // the loop's m-- and m++ of the upstream cancel: the same position is looked at again
        }
        let mut p = self.r_actives;
        while p < self.ignored_pos {
            if self.accel[self.aii[p]] < 0.0 {
                self.rotate_to_end(p);
                self.ignored_pos -= 1;
            } else {
                p += 1;
            }
        }
        count
    }

    /// Sorts the active variables by decreasing solution (0x10035250).
    fn sort_actives_by_x(&mut self) {
        for i in 1..self.r_actives {
            let mut j = i;
            while j > 0 && self.x[self.aii[j]] > self.x[self.aii[j - 1]] {
                self.swap_pos(j, j - 1);
                j -= 1;
            }
        }
    }

    /// Restarts the solution from the current active set (0x100354e0). False when the restart fails.
    fn full_setup(&mut self) -> bool {
        loop {
            for v in self.reset_x.iter_mut() {
                *v = 0.0;
            }
            self.little_random_permutation();
            let m = self.r_actives;
            for i in 0..m {
                self.sx[i] = self.b[self.aii[i]];
            }
            if self.setup_lu() {
                self.status = 0;
                let sx = self.sx.clone();
                let mut out = std::mem::take(&mut self.delta_accel);
                self.lu_solve(&sx, &mut out);
                for i in 0..m {
                    self.reset_x[self.aii[i]] = out[i];
                }
                self.delta_accel = out;
            } else {
                self.status = 2;
                self.sort_actives_by_x();
                self.little_random_permutation();
                self.active_matrix(m);
                for i in 0..m {
                    self.sb[i] = self.b[self.aii[i]];
                }
                let mut out = std::mem::take(&mut self.delta_accel);
                let ok = gauss_solve(m, &mut self.sub, &mut self.sb, &mut out, GAUSS_EPS);
                if ok != 1 {
                    self.delta_accel = out;
                    return false;
                }
                for i in 0..m {
                    self.reset_x[self.aii[i]] = out[i];
                }
                self.delta_accel = out;
            }
            self.recompute_x_accel();
            let k = self.remove_negatives();
            if k < 1 {
                return true;
            }
        }
    }

    /// The warm start with the first `nfirst` variables active (0x10035380).
    fn start_initialize(&mut self, nfirst: usize) {
        self.r_actives = nfirst;
        self.ignored_pos = nfirst;
        for v in self.reset_x.iter_mut() {
            *v = 0.0;
        }
        if self.setup_lu() {
            self.status = 0;
            loop {
                let m = self.r_actives;
                for i in 0..m {
                    self.sx[i] = self.b[self.aii[i]];
                }
                let sx = self.sx.clone();
                let mut out = std::mem::take(&mut self.delta_accel);
                self.lu_solve(&sx, &mut out);
                for i in 0..m {
                    self.reset_x[self.aii[i]] = out[i];
                }
                self.delta_accel = out;
                self.recompute_x_accel();
                // the last active variable that went negative
                let mut neg: Option<usize> = None;
                for i in (0..m).rev() {
                    if self.x[self.aii[i]] < 0.0 {
                        neg = Some(i);
                        break;
                    }
                }
                let Some(neg) = neg else { return };
                let v = self.aii[neg];
                self.x[v] = 0.0;
                self.reset_x[v] = 0.0;
                self.r_actives -= 1;
                self.ignored_pos -= 1;
                let last = self.r_actives;
                self.swap_pos(last, neg);
                self.lu_remove();
                if self.status > 0 {
                    break;
                }
            }
            for v in self.accel.iter_mut() {
                *v = 0.0;
            }
            for v in self.x.iter_mut() {
                *v = 0.0;
            }
        }
        self.r_actives = 0;
        self.ignored_pos = 0;
    }

    /// The full test of the solution (0x100343a0): the complementarity of every variable within the maximum error.
    fn full_test(&self) -> bool {
        let n = self.n;
        for p in 0..n {
            let v = self.aii[p];
            let mut s = 0.0;
            for c in (0..n).rev() {
                s += self.af(v, c) * self.x[c];
            }
            s -= self.b[v];
            let bad = if p < self.r_actives {
                s.abs() > MAX_ERROR
            } else {
                (s - self.accel[v]).abs() > MAX_ERROR
            };
            if bad {
                return false;
            }
        }
        true
    }

    /// The direction a unit increase of variable `ig` takes the active variables (0x10034ff0). 0 when the solve fails.
    fn get_fx_dx(&mut self, ig: usize) -> i32 {
        for v in self.delta_f.iter_mut() {
            *v = 0.0;
        }
        let m = self.r_actives;
        if m == 0 {
            return 1;
        }
        for i in 0..m {
            self.sx[i] = -self.af(self.aii[i], ig);
        }
        let sx = self.sx.clone();
        let mut out = std::mem::take(&mut self.reset_accel);
        let ret = if self.status == 0 {
            let mut r = 1;
            if self.lu_solve(&sx, &mut out) == 0 {
                self.active_matrix(m);
                self.sb[..m].copy_from_slice(&sx[..m]);
                r = gauss_solve(m, &mut self.sub, &mut self.sb, &mut out, GAUSS_EPS);
            }
            r
        } else {
            self.active_matrix(m);
            self.sb[..m].copy_from_slice(&sx[..m]);
            gauss_solve(m, &mut self.sub, &mut self.sb, &mut out, GAUSS_EPS)
        };
        for i in 0..m {
            self.delta_f[self.aii[i]] = out[i];
        }
        self.reset_accel = out;
        self.delta_f[ig] = 1.0;
        ret
    }

    /// The change of the ignored accelerations for the direction of `delta_f` (0x100342c0).
    fn compute_delta_accel(&mut self) {
        let ig = self.aii[self.ignored_pos];
        for p in self.r_actives..self.n {
            let v = self.aii[p];
            let mut s = 0.0;
            for m in 0..self.r_actives {
                s += self.delta_f[self.aii[m]] * self.af(v, self.aii[m]);
            }
            self.delta_accel[v] = s + self.af(v, ig);
        }
    }

    /// Moves the solution and the accelerations by `s` along the direction (0x10034f40).
    fn update_step(&mut self, s: f64) {
        for i in 0..self.n {
            self.accel[i] += s * self.delta_accel[i];
            self.x[i] += s * self.delta_f[i];
        }
        for p in self.r_actives..self.ignored_pos {
            let v = self.aii[p];
            if self.accel[v] < 0.0 {
                self.accel[v] = 0.0;
            }
        }
        for p in 0..self.r_actives {
            let v = self.aii[p];
            if self.x[v] < 0.0 {
                self.x[v] = 0.0;
            }
        }
    }

    /// Drops the active variables that reached zero (0x10034a40).
    fn prune_actives(&mut self) {
        let mut m = 0usize;
        while m < self.r_actives {
            let v = self.aii[m];
            if self.x[v] < TEST_EPS {
                self.x[v] = 0.0;
                let last = self.r_actives - 1;
                self.swap_pos(m, last);
                self.r_actives -= 1;
                self.lu_remove();
            } else {
                m += 1;
            }
        }
    }

    /// The active-set walk (0x10034ab0). Returns true when solved.
    fn solve_lc(&mut self) -> bool {
        let n = self.n;
        let eps = TEST_EPS;
        let mut steps = 0u32;
        let mut zero_steps = 0usize;
        let mut did_real = false;
        let mut countdown = 7i32;
        let mut step = 0.0f64;
        let mut jpos: i64 = 0;
        let mut ig = 0usize;
        let mut st = St::Top;
        loop {
            match st {
                St::Top => {
                    if did_real {
                        steps += 1;
                    }
                    if steps > MAX_LCP_STEPS {
                        return false;
                    }
                    if countdown == 0 {
                        if !self.full_test() {
                            st = St::Restart;
                            continue;
                        }
                        countdown = 7;
                    } else {
                        countdown -= 1;
                    }
                    st = St::Main;
                }
                St::Main => {
                    if self.ignored_pos >= n {
                        return true;
                    }
                    ig = self.aii[self.ignored_pos];
                    if self.status > 0 {
                        if self.status == 1 && did_real {
                            self.little_random_permutation();
                            if self.setup_lu() {
                                self.status = 0;
                            }
                        } else {
                            self.status = 1;
                        }
                    }
                    if self.accel[ig].abs() < eps {
                        if self.x[ig].abs() < eps {
                            st = St::ToInactive;
                        } else {
                            jpos = self.ignored_pos as i64;
                            st = St::IgToActive;
                        }
                        continue;
                    }
                    if self.accel[ig] >= 0.0 {
                        st = St::ToInactive;
                        continue;
                    }
                    // the contact is still approaching: increase x[ig]
                    if self.get_fx_dx(ig) == 0 {
                        st = St::Restart;
                        continue;
                    }
                    self.delta_f[ig] = 1.0;
                    self.compute_delta_accel();
                    for m in 0..self.r_actives {
                        let v = self.aii[m];
                        self.delta_accel[v] = 0.0;
                    }
                    if MAX_STEP * self.delta_accel[ig] <= -self.accel[ig] {
                        jpos = -1;
                        step = 1e101;
                    } else {
                        jpos = self.ignored_pos as i64;
                        step = -self.accel[ig] / self.delta_accel[ig];
                    }
                    let mut early = false;
                    for m in 0..self.r_actives {
                        let v = self.aii[m];
                        let d = self.delta_f[v];
                        if d < -eps {
                            let t = -(self.x[v] / d);
                            if t.abs() < eps && self.x[v] < eps {
                                jpos = m as i64;
                                step = t;
                                early = true;
                                break;
                            }
                            if t < step + eps {
                                step = t;
                                jpos = m as i64;
                            }
                        }
                    }
                    if !early {
                        for m in self.r_actives..self.ignored_pos {
                            let v = self.aii[m];
                            let d = self.delta_accel[v];
                            if d < -TEST_EPS {
                                let t = -(self.accel[v] / d);
                                if t < step - eps {
                                    step = t;
                                    jpos = m as i64;
                                }
                            }
                        }
                        if step < 0.0 {
                            step = 0.0;
                        }
                        if jpos < 0 {
                            return false;
                        }
                    }
                    st = St::StepCheck;
                }
                St::StepCheck => {
                    if step > MAX_STEP {
                        st = St::Restart;
                        continue;
                    }
                    if step < TEST_EPS {
                        zero_steps += 1;
                        if zero_steps > (n >> 1) + 2 {
                            st = St::Restart;
                            continue;
                        }
                    } else {
                        zero_steps = 0;
                    }
                    did_real = true;
                    self.update_step(step);
                    let jp = jpos as usize;
                    if jp < self.r_actives {
                        // an active variable reached x = 0: it goes inactive
                        let v = self.aii[jp];
                        self.x[v] = 0.0;
                        self.r_actives -= 1;
                        let last = self.r_actives;
                        self.swap_pos(last, jp);
                        self.lu_remove();
                        st = St::Top;
                        continue;
                    }
                    if jp < self.ignored_pos {
                        // an inactive variable reached accel = 0: it goes active
                        if step > TEST_EPS {
                            self.prune_actives();
                        }
                        let v = self.aii[jp];
                        self.accel[v] = 0.0;
                        let ra = self.r_actives;
                        self.swap_pos(ra, jp);
                        self.r_actives += 1;
                        self.lu_add();
                        st = St::Top;
                        continue;
                    }
                    st = St::IgToActive;
                }
                St::IgToActive => {
                    self.prune_actives();
                    let v = self.aii[jpos as usize];
                    self.accel[v] = 0.0;
                    if self.x[v] < 0.0 {
                        self.x[v] = 0.0;
                    }
                    let (ra, jp) = (self.r_actives, jpos as usize);
                    self.swap_pos(ra, jp);
                    self.ignored_pos += 1;
                    self.r_actives += 1;
                    if self.ignored_pos < n {
                        self.lu_add();
                        st = St::Top;
                        continue;
                    }
                    countdown = 0;
                    st = St::Top;
                }
                St::ToInactive => {
                    did_real = false;
                    self.x[ig] = 0.0;
                    if self.accel[ig] < 0.0 {
                        self.accel[ig] = 0.0;
                    }
                    self.ignored_pos += 1;
                    if self.ignored_pos >= n {
                        countdown = 0;
                    }
                    st = St::Top;
                }
                St::Restart => {
                    steps += 1;
                    if steps > MAX_LCP_STEPS {
                        return false;
                    }
                    zero_steps = 0;
                    if !self.full_setup() {
                        return false;
                    }
                    countdown = 7;
                    st = St::Main;
                }
            }
        }
    }
}

/// The states of the active-set walk (the labels of solve_lc).
#[derive(Clone, Copy)]
enum St {
    Top,
    Main,
    StepCheck,
    IgToActive,
    ToInactive,
    Restart,
}

/// Solves A x + b >= 0 complementary to x >= 0 from the warm start with the first `nfirst` variables active
/// (`lcs_init_and_solve`, 0x100346e0). Returns true when solved; x holds the solution.
pub fn lcp_solve(a: &[f64], b: &[f64], x: &mut [f64], n: usize, nfirst: usize) -> bool {
    let mut l = Lcs {
        n,
        a,
        b,
        x: vec![0.0; n],
        accel: vec![0.0; n],
        delta_f: vec![0.0; n],
        delta_accel: vec![0.0; n],
        reset_x: vec![0.0; n],
        reset_accel: vec![0.0; n],
        aii: (0..n).collect(),
        found: (0..n).collect(),
        r_actives: 0,
        ignored_pos: 0,
        p1: 0,
        p2: 0,
        q1: 0,
        q2: 0,
        status: 0,
        sub: vec![0.0; n * n],
        sb: vec![0.0; n],
        sx: vec![0.0; n],
    };
    for i in 0..n {
        l.accel[i] = -b[i];
    }
    l.start_initialize(nfirst);
    let ok = l.solve_lc();
    if ok {
        x[..n].copy_from_slice(&l.x[..n]);
    }
    ok
}

/// One core's share of a two-direction reaction (`IvpCoreReaction`, LS5): the directions, the core-space reaction of
/// each direction, its mass-weighted version, and the 2 x 2 matrix of the pair of directions.
#[derive(Clone, Debug, Default)]
pub struct CoreReaction {
    pub dir: [[f32; 3]; 2],
    pub nd: usize,
    /// [core][dir] : (r x dir) in core space and its weighted version, with the mass row as the fourth entry.
    pub cr_out: [[[f32; 4]; 2]; 2],
    pub cr_mult: [[[f32; 4]; 2]; 2],
    pub m00: f64,
    pub m01: f64,
    pub m11: f64,
    pub dv: [f32; 2],
}

/// Adds core `k`'s share of a reaction at point `p_ws` for the direction pair (`reaction_add_core`, 0x10033560). `core`
/// is the core's world pose and speeds: its rotation, centre, inertia, mass and speeds.
pub fn reaction_add_core(
    t: &mut CoreReaction,
    k: usize,
    core: &crate::core::Core,
    p_ws: [f64; 3],
    sign: f32,
) {
    let r: [f32; 3] = [
        (p_ws[0] - core.m_world_f_core.vv[0]) as f32,
        (p_ws[1] - core.m_world_f_core.vv[1]) as f32,
        (p_ws[2] - core.m_world_f_core.vv[2]) as f32,
    ];
    let nd = t.nd;
    for d in 0..nd {
        let dir = t.dir[d];
        // x = r x dir (f32), then the core-space rotation of it (f64, stored f32)
        let x = [
            r[1] * dir[2] - r[2] * dir[1],
            r[2] * dir[0] - r[0] * dir[2],
            r[0] * dir[1] - r[1] * dir[0],
        ];
        let rot = crate::math::rtmul(
            &core.m_world_f_core.r,
            &[f64::from(x[0]), f64::from(x[1]), f64::from(x[2])],
        );
        let out = [rot[0] as f32, rot[1] as f32, rot[2] as f32, 1.0f32];
        let mut mult = [0.0f32; 4];
        for i in 0..3 {
            mult[i] = out[i] * core.inv_rot_inertia[i];
        }
        mult[3] = core.inv_mass;
        let d4 = f64::from(out[0]) * f64::from(mult[0])
            + f64::from(out[1]) * f64::from(mult[1])
            + f64::from(out[2]) * f64::from(mult[2])
            + f64::from(out[3]) * f64::from(mult[3]);
        if d == 0 {
            t.m00 += d4;
        } else {
            t.m11 += d4;
            // the upstream's ivp_dotd: three components only
            let c0 = t.cr_out[k][0];
            t.m01 += f64::from(mult[2]) * f64::from(c0[2])
                + f64::from(mult[1]) * f64::from(c0[1])
                + f64::from(mult[0]) * f64::from(c0[0]);
        }
        t.cr_out[k][d] = out;
        t.cr_mult[k][d] = mult;
        let dv = f64::from(core.rot_speed[2]) * f64::from(out[2])
            + f64::from(core.rot_speed[1]) * f64::from(out[1])
            + f64::from(core.rot_speed[0]) * f64::from(out[0]);
        let sv = f64::from(dir[2]) * f64::from(core.speed[2])
            + f64::from(dir[1]) * f64::from(core.speed[1])
            + f64::from(dir[0]) * f64::from(core.speed[0]);
        t.dv[d] = (f64::from(t.dv[d]) + f64::from(sign) * (dv + sv)) as f32;
    }
}

/// The reaction of a pair of cores for two directions at point `p_ws` (`ivp_core_reaction_init`, 0x10033a30).
pub fn core_reaction_init(
    c0: Option<&crate::core::Core>,
    c1: Option<&crate::core::Core>,
    p_ws: [f64; 3],
    d0: [f32; 3],
    d1: [f32; 3],
) -> CoreReaction {
    let mut t = CoreReaction {
        dir: [d0, d1],
        nd: 2,
        ..CoreReaction::default()
    };
    if let Some(c) = c0 {
        reaction_add_core(&mut t, 0, c, p_ws, 1.0);
    }
    if let Some(c) = c1 {
        reaction_add_core(&mut t, 1, c, p_ws, -1.0);
    }
    t
}

/// Pushes the cores by impulses `imp` along the two directions, straight into the speeds (`ivp_core_reaction_push2`,
/// 0x10033ad0). Returns the cores' changes as (speed, rot speed) for core 0 and core 1 in the caller's order.
pub fn core_reaction_push2(
    t: &CoreReaction,
    sign_side: [f32; 2],
    inv_mass: [f64; 2],
    imp: [f32; 2],
    speed: &mut [[f32; 3]; 2],
    rot: &mut [[f32; 3]; 2],
) {
    for k in 0..2 {
        let sg = sign_side[k];
        let im = inv_mass[k];
        for d in 0..2 {
            let s = f64::from(sg * imp[d]) * im;
            for i in 0..3 {
                speed[k][i] = (f64::from(speed[k][i]) + f64::from(t.dir[d][i]) * s) as f32;
            }
        }
        for d in 0..2 {
            for i in 0..3 {
                rot[k][i] = (f64::from(rot[k][i])
                    + f64::from(t.cr_mult[k][d][i]) * f64::from(sg * imp[d]))
                    as f32;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn gauss_solves_a_small_system() {
        // 2x + y = 5, x + 3y = 10  ->  x = 1, y = 3
        let mut a = vec![2.0, 1.0, 1.0, 3.0];
        let mut b = vec![5.0, 10.0];
        let mut x = vec![0.0; 2];
        assert_eq!(gauss_solve(2, &mut a, &mut b, &mut x, 1e-12), 1);
        assert!(
            (x[0] - 1.0).abs() < 1e-9 && (x[1] - 3.0).abs() < 1e-9,
            "{x:?}"
        );
    }

    #[test]
    fn lcp_of_a_single_contact_pushes_just_enough() {
        // one contact: the complementarity is A x - b >= 0 (the upstream's convention: accel starts at -b),
        // so 2x - 4 >= 0 with equality at the solution: x = 2
        let a = vec![2.0];
        let b = vec![4.0];
        let mut x = vec![0.0];
        assert!(lcp_solve(&a, &b, &mut x, 1, 0));
        assert!((x[0] - 2.0).abs() < 1e-6, "{x:?}");
    }

    #[test]
    fn lcp_leaves_a_separating_contact_at_zero() {
        // A x - b >= 0 already holds at x = 0 (x + 3 >= 0): nothing is pushed
        let a = vec![1.0];
        let b = vec![-3.0];
        let mut x = vec![0.0];
        assert!(lcp_solve(&a, &b, &mut x, 1, 0));
        assert!(x[0].abs() < 1e-9, "{x:?}");
    }
}
