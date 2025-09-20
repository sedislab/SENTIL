/-
We prove that the front of a monotonic deque over a sliding window of samples equals the minimum of a naive filter over the same window
-/

class VOrd (α : Type) where
  le : α → α → Prop
  decLe : (a b : α) → Decidable (le a b)
  le_refl : ∀ a, le a a
  le_trans : ∀ {a b c}, le a b → le b c → le a c
  le_antisymm : ∀ {a b}, le a b → le b a → a = b
  le_total : ∀ a b, le a b ∨ le b a

attribute [reducible, instance] VOrd.decLe

namespace VOrd
variable {α : Type} [VOrd α]

/-- reducible so its decidability follows from `decLe`. -/
@[reducible] def lt (a b : α) : Prop := ¬ le b a

theorem le_of_not_lt {a b : α} (h : ¬ lt a b) : le b a :=
  match decLe b a with
  | isTrue hle => hle
  | isFalse hnle => absurd hnle h

theorem le_of_lt {a b : α} (h : lt a b) : le a b := by
  rcases le_total a b with h' | h'
  · exact h'
  · exact absurd h' h

def omin (a b : α) : α := if le a b then a else b

theorem omin_le_left (a b : α) : le (omin a b) a := by
  unfold omin
  split
  next _ => exact le_refl a
  next h =>
    rcases le_total a b with h' | h'
    · exact absurd h' h
    · exact h'

theorem omin_le_right (a b : α) : le (omin a b) b := by
  unfold omin
  split
  next h => exact h
  next _ => exact le_refl b

theorem le_omin {a b c : α} (hca : le c a) (hcb : le c b) : le c (omin a b) := by
  unfold omin
  split
  · exact hca
  · exact hcb

theorem omin_eq_left {a b : α} (h : le a b) : omin a b = a := by
  unfold omin; exact if_pos h

instance (priority := low) decEq : DecidableEq α := fun a b =>
  if hab : le a b then
    if hba : le b a then isTrue (le_antisymm hab hba)
    else isFalse (by intro h; subst h; exact absurd (le_refl _) hba)
  else isFalse (by intro h; subst h; exact absurd (le_refl _) hab)

end VOrd

open VOrd

local infix:50 " ≤ᵥ " => VOrd.le
local infix:50 " <ᵥ " => VOrd.lt

structure Sample (α : Type) where
  time : Nat
  value : α
deriving DecidableEq, Repr, Inhabited

abbrev Deque (α : Type) := List (Sample α)

section Time
variable {α : Type}

def popFront : Deque α → Nat → Deque α
  | [], _ => []
  | x :: xs, cutoff =>
    if x.time < cutoff then popFront xs cutoff else x :: xs

inductive TimeIncr : Deque α → Prop
  | nil  : TimeIncr []
  | one  (x) : TimeIncr [x]
  | cons (x y rest) (hlt : x.time < y.time)
         (htail : TimeIncr (y :: rest)) : TimeIncr (x :: y :: rest)

theorem ti_tail (x : Sample α) (xs : Deque α) (h : TimeIncr (x :: xs)) :
    TimeIncr xs := by
  cases h with | one => exact TimeIncr.nil | cons _ _ _ _ ht => exact ht

theorem ti_head_lt (x : Sample α) (xs : Deque α) (h : TimeIncr (x :: xs)) :
    ∀ s ∈ xs, x.time < s.time := by
  induction xs generalizing x with
  | nil => intro s hs; nomatch hs
  | cons y ys ih =>
    intro s hs
    cases h with
    | cons _ _ _ hlt htail =>
      cases hs with
      | head => exact hlt
      | tail _ hs' => exact Nat.lt_trans hlt (ih y htail s hs')

theorem ti_head_le_time (x : Sample α) (xs : Deque α) (h : TimeIncr (x :: xs)) :
    ∀ s ∈ (x :: xs), x.time ≤ s.time := by
  intro s hs
  cases hs with
  | head => exact Nat.le_refl _
  | tail _ hs' => exact Nat.le_of_lt (ti_head_lt x xs h s hs')

theorem popFront_subset (xs : Deque α) (c : Nat) (s : Sample α) :
    s ∈ popFront xs c → s ∈ xs := by
  induction xs with
  | nil => intro h; nomatch h
  | cons x xs ih =>
    unfold popFront
    split
    next _ => intro h; exact List.Mem.tail _ (ih h)
    next _ => intro h; exact h

theorem popFront_time_ge (xs : Deque α) (c : Nat) (h : TimeIncr xs) (s : Sample α) :
    s ∈ popFront xs c → s.time ≥ c := by
  induction xs with
  | nil => intro h'; nomatch h'
  | cons x xs ih =>
    unfold popFront
    split
    next _ => exact ih (ti_tail x xs h)
    next hge =>
      intro hmem
      have hx_ge : x.time ≥ c := Nat.le_of_not_lt hge
      exact Nat.le_trans hx_ge (ti_head_le_time x xs h s hmem)

theorem popFront_complete (xs : Deque α) (c : Nat) (h : TimeIncr xs) (s : Sample α) :
    s ∈ xs → s.time ≥ c → s ∈ popFront xs c := by
  induction xs with
  | nil => intro h'; nomatch h'
  | cons x xs ih =>
    intro hmem hge
    unfold popFront
    split
    next hlt =>
      cases hmem with
      | head => exact absurd hge (Nat.not_le.mpr hlt)
      | tail _ h' => exact ih (ti_tail x xs h) h' hge
    next _ => exact hmem

theorem popFront_ti (xs : Deque α) (c : Nat) (h : TimeIncr xs) :
    TimeIncr (popFront xs c) := by
  induction xs with
  | nil => exact TimeIncr.nil
  | cons x xs ih =>
    unfold popFront
    split
    next _ => exact ih (ti_tail x xs h)
    next _ => exact h

theorem append_ti (xs : Deque α) (s : Sample α)
    (h : TimeIncr xs) (hgt : ∀ x ∈ xs, x.time < s.time) :
    TimeIncr (xs ++ [s]) := by
  induction xs with
  | nil => exact TimeIncr.one s
  | cons x xs ih =>
    have htail := ti_tail x xs h
    have hgt_tail : ∀ y ∈ xs, y.time < s.time := fun y hy => hgt y (List.Mem.tail _ hy)
    cases xs with
    | nil =>
      exact TimeIncr.cons x s [] (hgt x (List.Mem.head _)) (TimeIncr.one s)
    | cons y ys =>
      cases h with
      | cons _ _ _ hlt htail' =>
        exact TimeIncr.cons x y (ys ++ [s]) hlt (ih htail' hgt_tail)

/-- Opaque so `Nat.sub` does not normalize away the lower bound. -/
@[irreducible] def windowPred (t w : Nat) (s : Sample α) : Bool :=
  decide (t - w ≤ s.time) && decide (s.time ≤ t)

@[simp] theorem windowPred_iff (t w : Nat) (s : Sample α) :
    windowPred t w s = true ↔ (t - w ≤ s.time ∧ s.time ≤ t) := by
  unfold windowPred
  simp [Bool.and_eq_true, decide_eq_true_eq]

theorem filter_mem_stream {s : Sample α} {stream : Deque α} {t w : Nat}
    (h : s ∈ stream.filter (windowPred t w)) : s ∈ stream :=
  (List.mem_filter.mp h).1

theorem filter_mem_window {s : Sample α} {stream : Deque α} {t w : Nat}
    (h : s ∈ stream.filter (windowPred t w)) : t - w ≤ s.time ∧ s.time ≤ t :=
  (windowPred_iff t w s).mp (List.mem_filter.mp h).2

theorem mem_filter_of_window {s : Sample α} {stream : Deque α} {t w : Nat}
    (hin : s ∈ stream) (hlo : t - w ≤ s.time) (hhi : s.time ≤ t) :
    s ∈ stream.filter (windowPred t w) :=
  List.mem_filter.mpr ⟨hin, (windowPred_iff t w s).mpr ⟨hlo, hhi⟩⟩

end Time

section Order
variable {α : Type} [VOrd α]

def popBack : Deque α → α → Deque α
  | [], _ => []
  | x :: xs, v =>
    if x.value <ᵥ v then x :: popBack xs v else popBack xs v

def processStep (d : Deque α) (s : Sample α) (w : Nat) : Deque α :=
  popBack (popFront d (s.time - w)) s.value ++ [s]

inductive ValueNonDecr : Deque α → Prop
  | nil  : ValueNonDecr []
  | one  (x) : ValueNonDecr [x]
  | cons (x y rest) (hle : x.value ≤ᵥ y.value)
         (htail : ValueNonDecr (y :: rest)) : ValueNonDecr (x :: y :: rest)

theorem vnd_tail (x : Sample α) (xs : Deque α) (h : ValueNonDecr (x :: xs)) :
    ValueNonDecr xs := by
  cases h with | one => exact ValueNonDecr.nil | cons _ _ _ _ ht => exact ht

theorem vnd_head_le (x : Sample α) (xs : Deque α) (h : ValueNonDecr (x :: xs)) :
    ∀ s ∈ xs, x.value ≤ᵥ s.value := by
  induction xs generalizing x with
  | nil => intro s hs; nomatch hs
  | cons y ys ih =>
    intro s hs
    cases h with
    | cons _ _ _ hle htail =>
      cases hs with
      | head => exact hle
      | tail _ hs' => exact VOrd.le_trans hle (ih y htail s hs')

theorem popBack_subset (xs : Deque α) (v : α) (s : Sample α) :
    s ∈ popBack xs v → s ∈ xs := by
  induction xs with
  | nil => intro h; nomatch h
  | cons x xs ih =>
    unfold popBack
    split
    next _ =>
      intro h; cases h with
      | head => exact List.Mem.head _
      | tail _ h' => exact List.Mem.tail _ (ih h')
    next _ =>
      intro h; exact List.Mem.tail _ (ih h)

theorem popBack_val_lt (xs : Deque α) (v : α) (s : Sample α) :
    s ∈ popBack xs v → s.value <ᵥ v := by
  induction xs with
  | nil => intro h; nomatch h
  | cons x xs ih =>
    unfold popBack
    split
    next hlt =>
      intro h; cases h with
      | head => exact hlt
      | tail _ h' => exact ih h'
    next _ =>
      intro h; exact ih h

theorem popBack_complete (xs : Deque α) (v : α) (s : Sample α) :
    s ∈ xs → s.value <ᵥ v → s ∈ popBack xs v := by
  induction xs with
  | nil => intro h; nomatch h
  | cons x xs ih =>
    intro hmem hval
    unfold popBack
    split
    next _ =>
      cases hmem with
      | head => exact List.Mem.head _
      | tail _ h' => exact List.Mem.tail _ (ih h' hval)
    next hge =>
      cases hmem with
      | head => exact absurd hval hge
      | tail _ h' => exact ih h' hval

theorem popBack_vnd (xs : Deque α) (v : α) (h : ValueNonDecr xs) :
    ValueNonDecr (popBack xs v) := by
  induction xs with
  | nil => exact ValueNonDecr.nil
  | cons x xs ih =>
    have htail := vnd_tail x xs h
    unfold popBack
    split
    next _ =>
      cases hpb : popBack xs v with
      | nil => exact ValueNonDecr.one x
      | cons y ys =>
        have hy_mem : y ∈ popBack xs v := hpb ▸ List.Mem.head _
        have hle := vnd_head_le x xs h y (popBack_subset xs v y hy_mem)
        exact ValueNonDecr.cons x y ys hle (hpb ▸ ih htail)
    next _ => exact ih htail

theorem popBack_ti (xs : Deque α) (v : α) (h : TimeIncr xs) :
    TimeIncr (popBack xs v) := by
  induction xs with
  | nil => exact TimeIncr.nil
  | cons x xs ih =>
    have htail := ti_tail x xs h
    unfold popBack
    split
    next _ =>
      cases hpb : popBack xs v with
      | nil => exact TimeIncr.one x
      | cons y ys =>
        have hy_mem : y ∈ popBack xs v := hpb ▸ List.Mem.head _
        have hlt_t := ti_head_lt x xs h y (popBack_subset xs v y hy_mem)
        exact TimeIncr.cons x y ys hlt_t (hpb ▸ ih htail)
    next _ => exact ih htail

theorem popFront_vnd (xs : Deque α) (c : Nat) (h : ValueNonDecr xs) :
    ValueNonDecr (popFront xs c) := by
  induction xs with
  | nil => exact ValueNonDecr.nil
  | cons x xs ih =>
    unfold popFront
    split
    next _ => exact ih (vnd_tail x xs h)
    next _ => exact h

theorem append_vnd (xs : Deque α) (s : Sample α)
    (h : ValueNonDecr xs) (hge : ∀ x ∈ xs, x.value ≤ᵥ s.value) :
    ValueNonDecr (xs ++ [s]) := by
  induction xs with
  | nil => exact ValueNonDecr.one s
  | cons x xs ih =>
    have htail := vnd_tail x xs h
    have hge_tail : ∀ y ∈ xs, y.value ≤ᵥ s.value := fun y hy => hge y (List.Mem.tail _ hy)
    cases xs with
    | nil =>
      exact ValueNonDecr.cons x s [] (hge x (List.Mem.head _)) (ValueNonDecr.one s)
    | cons y ys =>
      cases h with
      | cons _ _ _ hle htail' =>
        exact ValueNonDecr.cons x y (ys ++ [s]) hle (ih htail' hge_tail)

structure DequeInv (deque stream : Deque α) (t w : Nat) : Prop where
  vnd   : ValueNonDecr deque
  ti    : TimeIncr deque
  sub   : ∀ s ∈ deque, s ∈ stream
  inw   : ∀ s ∈ deque, t - w ≤ s.time ∧ s.time ≤ t
  cov   : ∀ s ∈ stream, t - w ≤ s.time → s.time ≤ t → s ∉ deque →
            ∃ s' ∈ deque, s'.time > s.time ∧ s'.value ≤ᵥ s.value
  bound : ∀ s ∈ stream, s.time ≤ t

theorem step_vnd (D : Deque α) (s : Sample α) (w : Nat) (h : ValueNonDecr D) :
    ValueNonDecr (processStep D s w) := by
  unfold processStep
  apply append_vnd
  · exact popBack_vnd _ _ (popFront_vnd D _ h)
  · intro x hx; exact VOrd.le_of_lt (popBack_val_lt _ _ _ hx)

theorem step_cov (D S : Deque α) (s_new : Sample α) (t_old w : Nat)
    (h_inv : DequeInv D S t_old w) (h_after : s_new.time > t_old) :
    ∀ s ∈ (S ++ [s_new]), s_new.time - w ≤ s.time → s.time ≤ s_new.time →
      s ∉ processStep D s_new w →
      ∃ s' ∈ processStep D s_new w, s'.time > s.time ∧ s'.value ≤ᵥ s.value := by
  intro s hs h_lo h_hi h_notin
  have h_snew_in : s_new ∈ processStep D s_new w :=
    List.mem_append.mpr (Or.inr (List.Mem.head _))
  rcases List.mem_append.mp hs with hs_old | hs_new
  ·
    have h_not_D'' : s ∉ popBack (popFront D (s_new.time - w)) s_new.value :=
      fun hmem => h_notin (List.mem_append.mpr (Or.inl hmem))
    have h_s_le_old : s.time ≤ t_old := h_inv.bound s hs_old
    have h_s_ge_old : t_old - w ≤ s.time := by omega
    by_cases h_in_D : s ∈ D
    ·
      have h_in_pf : s ∈ popFront D (s_new.time - w) :=
        popFront_complete D _ h_inv.ti s h_in_D (by omega)
      have h_val_ge : s_new.value ≤ᵥ s.value := by
        have h_not_lt : ¬ s.value <ᵥ s_new.value :=
          fun h_lt => h_not_D'' (popBack_complete _ _ s h_in_pf h_lt)
        exact le_of_not_lt h_not_lt
      exact ⟨s_new, h_snew_in, by omega, h_val_ge⟩
    ·
      obtain ⟨s', h_s'_D, h_s'_time, h_s'_val⟩ :=
        h_inv.cov s hs_old h_s_ge_old h_s_le_old h_in_D
      have h_s'_pf : s' ∈ popFront D (s_new.time - w) :=
        popFront_complete D _ h_inv.ti s' h_s'_D (by omega)
      cases decLe s_new.value s'.value with
      | isFalse h_s'_lt =>
        exact ⟨s', List.mem_append.mpr (Or.inl (popBack_complete _ _ s' h_s'_pf h_s'_lt)),
               h_s'_time, h_s'_val⟩
      | isTrue h_ge =>
        exact ⟨s_new, h_snew_in, by omega, VOrd.le_trans h_ge h_s'_val⟩
  ·
    cases hs_new with
    | head => exact absurd h_snew_in h_notin
    | tail _ h => nomatch h

theorem step_inv (D S : Deque α) (s_new : Sample α) (t_old w : Nat)
    (h_inv : DequeInv D S t_old w) (h_after : s_new.time > t_old) :
    DequeInv (processStep D s_new w) (S ++ [s_new]) s_new.time w := {
  vnd := step_vnd D s_new w h_inv.vnd
  ti := by
    unfold processStep
    apply append_ti
    · exact popBack_ti _ _ (popFront_ti D _ h_inv.ti)
    · intro x hx
      have h2 := popFront_subset _ _ _ (popBack_subset _ _ _ hx)
      exact Nat.lt_of_le_of_lt (h_inv.bound x (h_inv.sub x h2)) h_after
  sub := by
    intro x hx
    rcases List.mem_append.mp hx with h | h
    · exact List.mem_append.mpr (Or.inl
        (h_inv.sub x (popFront_subset _ _ _ (popBack_subset _ _ _ h))))
    · cases h with
      | head => exact List.mem_append.mpr (Or.inr (List.Mem.head _))
      | tail _ h' => nomatch h'
  inw := by
    intro x hx
    rcases List.mem_append.mp hx with h | h
    · have h1 := popBack_subset _ _ _ h
      have h2 := popFront_subset _ _ _ h1
      constructor
      · exact popFront_time_ge D _ h_inv.ti x h1
      · exact Nat.le_of_lt (Nat.lt_of_le_of_lt (h_inv.bound x (h_inv.sub x h2)) h_after)
    · cases h with
      | head => exact ⟨Nat.sub_le _ _, Nat.le_refl _⟩
      | tail _ h' => nomatch h'
  cov := step_cov D S s_new t_old w h_inv h_after
  bound := by
    intro x hx
    rcases List.mem_append.mp hx with h | h
    · exact Nat.le_of_lt (Nat.lt_of_le_of_lt (h_inv.bound x h) h_after)
    · cases h with
      | head => exact Nat.le_refl _
      | tail _ h' => nomatch h'
}

def sampleMin : Deque α → Option α
  | [] => none
  | x :: xs => some (xs.foldl (fun acc s => omin acc s.value) x.value)

def naiveWindowMin (stream : Deque α) (t w : Nat) : Option α :=
  sampleMin (stream.filter (windowPred t w))

theorem foldl_min_le_init (xs : List (Sample α)) (init : α) :
    xs.foldl (fun acc s => omin acc s.value) init ≤ᵥ init := by
  induction xs generalizing init with
  | nil => exact VOrd.le_refl _
  | cons x xs ih => exact VOrd.le_trans (ih _) (omin_le_left _ _)

theorem foldl_min_le_elem (xs : List (Sample α)) (init : α) (s : Sample α) (hs : s ∈ xs) :
    xs.foldl (fun acc s => omin acc s.value) init ≤ᵥ s.value := by
  induction xs generalizing init with
  | nil => nomatch hs
  | cons x xs ih =>
    cases hs with
    | head => exact VOrd.le_trans (foldl_min_le_init xs _) (omin_le_right _ _)
    | tail _ h => exact ih _ h

theorem foldl_min_ge (xs : List (Sample α)) (init v : α)
    (h_init : v ≤ᵥ init) (h_all : ∀ s ∈ xs, v ≤ᵥ s.value) :
    v ≤ᵥ xs.foldl (fun acc s => omin acc s.value) init := by
  induction xs generalizing init with
  | nil => exact h_init
  | cons x xs ih =>
    apply ih
    · exact le_omin h_init (h_all x (List.Mem.head _))
    · intro s hs; exact h_all s (List.Mem.tail _ hs)

theorem foldl_min_eq_init (xs : List (Sample α)) (init : α)
    (h : ∀ s ∈ xs, init ≤ᵥ s.value) :
    xs.foldl (fun acc s => omin acc s.value) init = init := by
  induction xs generalizing init with
  | nil => rfl
  | cons x xs ih =>
    show List.foldl _ (omin init x.value) xs = init
    rw [omin_eq_left (h x (List.Mem.head _))]
    exact ih init (fun s hs => h s (List.Mem.tail _ hs))

theorem front_eq_naiveMin (D S : Deque α) (t w : Nat) (h : DequeInv D S t w) :
    (D.head?.map (·.value)) = naiveWindowMin S t w := by
  cases hD : D with
  | nil =>
    simp only [List.head?, Option.map]
    show none = naiveWindowMin S t w
    unfold naiveWindowMin
    suffices hw : S.filter (windowPred t w) = [] by rw [hw]; rfl
    match hf : S.filter (windowPred t w) with
    | [] => rfl
    | s :: _ =>
      exfalso
      have hin := filter_mem_stream (hf ▸ List.Mem.head (α := Sample α) _)
      have ⟨hlo, hhi⟩ := filter_mem_window (hf ▸ List.Mem.head (α := Sample α) _)
      have h_notin : s ∉ D := fun hm => by rw [hD] at hm; nomatch hm
      obtain ⟨s', hs', _, _⟩ := h.cov s hin hlo hhi h_notin
      rw [hD] at hs'; nomatch hs'
  | cons x rest =>
    simp only [List.head?, Option.map]
    have h_vnd : ValueNonDecr (x :: rest) := hD ▸ h.vnd
    have h_x_S : x ∈ S := h.sub x (hD ▸ List.Mem.head _)
    have ⟨hlo, hhi⟩ := h.inw x (hD ▸ List.Mem.head _)
    have h_x_W : x ∈ S.filter (windowPred t w) := mem_filter_of_window h_x_S hlo hhi
    have h_all_ge : ∀ s ∈ S.filter (windowPred t w), x.value ≤ᵥ s.value := by
      intro s hs
      have hin := filter_mem_stream hs
      have ⟨hlo', hhi'⟩ := filter_mem_window hs
      by_cases h_in_D : s ∈ D
      · rw [hD] at h_in_D
        cases h_in_D with
        | head => exact VOrd.le_refl _
        | tail _ h' => exact vnd_head_le x rest h_vnd s h'
      · obtain ⟨s', hs'D, _, hs'val⟩ := h.cov s hin hlo' hhi' h_in_D
        rw [hD] at hs'D
        cases hs'D with
        | head => exact hs'val
        | tail _ h' => exact VOrd.le_trans (vnd_head_le x rest h_vnd s' h') hs'val
    show some x.value = naiveWindowMin S t w
    unfold naiveWindowMin
    cases hW : S.filter (windowPred t w) with
    | nil => exact absurd hW (List.ne_nil_of_mem h_x_W)
    | cons w0 wrest =>
      unfold sampleMin
      congr 1
      apply VOrd.le_antisymm
      ·
        apply foldl_min_ge
        · exact h_all_ge w0 (hW ▸ List.Mem.head _)
        · intro s hs; exact h_all_ge s (hW ▸ List.Mem.tail _ hs)
      ·
        rw [hW] at h_x_W
        cases h_x_W with
        | head => exact foldl_min_le_init wrest x.value
        | tail _ h' => exact foldl_min_le_elem wrest w0.value x h'

def processN (stream : Deque α) (w : Nat) : Deque α :=
  stream.foldl (fun d s => processStep d s w) []

/-- The final time comes back existentially to avoid matching on `getLast?`. -/
theorem foldl_inv (stream D S : Deque α) (t w : Nat)
    (h_inv : DequeInv D S t w)
    (h_ti : TimeIncr stream)
    (h_above : ∀ s ∈ stream, s.time > t) :
    ∃ t_final,
      DequeInv (stream.foldl (fun d s => processStep d s w) D) (S ++ stream) t_final w ∧
      (stream = [] → t_final = t) ∧
      (∀ slast, stream.getLast? = some slast → t_final = slast.time) := by
  induction stream generalizing D S t with
  | nil =>
    refine ⟨t, ?_, fun _ => rfl, fun _ h => nomatch h⟩
    simpa using h_inv
  | cons s rest ih =>
    simp only [List.foldl_cons]
    have h_s_gt : s.time > t := h_above s (List.Mem.head _)
    have h_new := step_inv D S s t w h_inv h_s_gt
    have h_ti_rest := ti_tail s rest h_ti
    have h_rest_above : ∀ r ∈ rest, r.time > s.time :=
      fun r hr => ti_head_lt s rest h_ti r hr
    obtain ⟨t_f, h_inv_f, h_nil, h_last⟩ :=
      ih (processStep D s w) (S ++ [s]) s.time h_new h_ti_rest h_rest_above
    refine ⟨t_f, ?_, ?_, ?_⟩
    ·
      rwa [List.append_assoc, List.singleton_append] at h_inv_f
    ·
      intro h; simp at h
    ·
      intro slast h_last_eq
      cases rest with
      | nil =>
        simp [List.getLast?] at h_last_eq
        rw [h_nil rfl]
        exact congrArg (·.time) h_last_eq
      | cons r rs =>
        have : (s :: r :: rs).getLast? = (r :: rs).getLast? := by simp [List.getLast?]
        rw [this] at h_last_eq
        exact h_last slast h_last_eq

theorem deque_window_correct (stream : Deque α) (w : Nat)
    (h_ti : TimeIncr stream) (hne : stream ≠ []) :
    let D := processN stream w
    let t := (stream.getLast hne).time
    (D.head?.map (·.value)) = naiveWindowMin stream t w := by
  simp only [processN]
  obtain ⟨s0, rest, rfl⟩ := List.exists_cons_of_ne_nil hne
  · simp only [List.foldl_cons]
    have h_step0 : processStep ([] : Deque α) s0 w = [s0] := by
      unfold processStep popFront popBack; rfl
    rw [h_step0]
    have h_inv0 : DequeInv [s0] [s0] s0.time w := {
      vnd   := ValueNonDecr.one s0
      ti    := TimeIncr.one s0
      sub   := by intro x hx; cases hx with | head => exact List.Mem.head _ | tail _ h => nomatch h
      inw   := by intro x hx; cases hx with
                  | head => exact ⟨Nat.sub_le _ _, Nat.le_refl _⟩ | tail _ h => nomatch h
      cov   := by intro s hs _ _ hn; cases hs with
                  | head => exact absurd (List.Mem.head _) hn | tail _ h => nomatch h
      bound := by intro s hs; cases hs with | head => exact Nat.le_refl _ | tail _ h => nomatch h
    }
    have h_ti_full : TimeIncr (s0 :: rest) := h_ti
    have h_ti_rest := ti_tail s0 rest h_ti_full
    have h_rest_above : ∀ r ∈ rest, r.time > s0.time :=
      fun r hr => ti_head_lt s0 rest h_ti_full r hr
    obtain ⟨t_f, h_inv_f, h_nil_f, h_last_f⟩ :=
      foldl_inv rest [s0] [s0] s0.time w h_inv0 h_ti_rest h_rest_above
    rw [List.singleton_append] at h_inv_f
    have h_tf_eq : t_f = ((s0 :: rest).getLast hne).time := by
      cases rest with
      | nil =>
        simp [List.getLast]
        exact h_nil_f rfl
      | cons r rs =>
        have h_eq : (s0 :: r :: rs).getLast (by simp) = (r :: rs).getLast (by simp) :=
          List.getLast_cons (by simp)
        rw [h_eq]
        exact h_last_f _ (List.getLast?_eq_some_getLast (by simp))
    rw [← h_tf_eq]
    exact front_eq_naiveMin _ (s0 :: rest) t_f w h_inv_f

end Order

instance NatOrd : VOrd Nat where
  le := Nat.le
  decLe := Nat.decLe
  le_refl := Nat.le_refl
  le_trans := Nat.le_trans
  le_antisymm := Nat.le_antisymm
  le_total := Nat.le_total

/-- the `always` monitor -/
theorem deque_sliding_window_min_correct (stream : Deque Nat) (w : Nat)
    (h_ti : TimeIncr stream) (hne : stream ≠ []) :
    ((processN stream w).head?.map (·.value))
      = naiveWindowMin stream ((stream.getLast hne).time) w :=
  deque_window_correct stream w h_ti hne

@[reducible] def NatOrdDual : VOrd Nat where
  le a b := Nat.le b a
  decLe a b := Nat.decLe b a
  le_refl a := Nat.le_refl a
  le_trans hab hbc := Nat.le_trans hbc hab
  le_antisymm hab hba := Nat.le_antisymm hba hab
  le_total a b := Nat.le_total b a

/-- Windowed maximum (the `eventually` monitor) -/
theorem deque_sliding_window_max_correct (stream : Deque Nat) (w : Nat)
    (h_ti : TimeIncr stream) (hne : stream ≠ []) :
    ((@processN Nat NatOrdDual stream w).head?.map (·.value))
      = @naiveWindowMin Nat NatOrdDual stream ((stream.getLast hne).time) w :=
  @deque_window_correct Nat NatOrdDual stream w h_ti hne

namespace Exec

structure MinDeque where
  data : Array (Sample Nat)
  deriving Repr

def MinDeque.empty : MinDeque := ⟨#[]⟩
def MinDeque.front (d : MinDeque) : Option Nat := d.data[0]?.map (·.value)

partial def popFrontImpl (arr : Array (Sample Nat)) (cutoff : Nat) : Array (Sample Nat) :=
  if h : arr.size > 0 then
    if (arr[0]'(by omega)).time < cutoff then
      popFrontImpl (arr.eraseIdx 0) cutoff
    else arr
  else arr

partial def popBackImpl (arr : Array (Sample Nat)) (v : Nat) : Array (Sample Nat) :=
  if arr.size > 0 then
    if arr.back!.value ≥ v then popBackImpl arr.pop v else arr
  else arr

def MinDeque.step (d : MinDeque) (s : Sample Nat) (w : Nat) : MinDeque :=
  ⟨(popBackImpl (popFrontImpl d.data (s.time - w)) s.value).push s⟩

def naiveMinAt (stream : Array (Sample Nat)) (t w : Nat) : Option Nat :=
  let active := stream.filter (fun s => decide (t - w ≤ s.time) && decide (s.time ≤ t))
  if h : active.size > 0
  then some (active.foldl (fun acc s => min acc s.value) (active[0]'(by omega)).value)
  else none

def verify (stream : Array (Sample Nat)) (w : Nat) : IO Unit := do
  let mut d := MinDeque.empty
  let mut ok := true
  for s in stream do
    d := d.step s w
    let dv := d.front
    let nv := naiveMinAt stream s.time w
    if dv != nv then
      IO.println s!"FAIL t={s.time}: deque={dv} naive={nv}"
      ok := false
  IO.println (if ok then "PASS" else "FAIL")

structure MaxDeque where
  data : Array (Sample Nat)
  deriving Repr

def MaxDeque.empty : MaxDeque := ⟨#[]⟩
def MaxDeque.front (d : MaxDeque) : Option Nat := d.data[0]?.map (·.value)

partial def popBackMaxImpl (arr : Array (Sample Nat)) (v : Nat) : Array (Sample Nat) :=
  if arr.size > 0 then
    if arr.back!.value ≤ v then popBackMaxImpl arr.pop v else arr
  else arr

def MaxDeque.step (d : MaxDeque) (s : Sample Nat) (w : Nat) : MaxDeque :=
  ⟨(popBackMaxImpl (popFrontImpl d.data (s.time - w)) s.value).push s⟩

def naiveMaxAt (stream : Array (Sample Nat)) (t w : Nat) : Option Nat :=
  let active := stream.filter (fun s => decide (t - w ≤ s.time) && decide (s.time ≤ t))
  if h : active.size > 0
  then some (active.foldl (fun acc s => max acc s.value) (active[0]'(by omega)).value)
  else none

def verifyMax (stream : Array (Sample Nat)) (w : Nat) : IO Unit := do
  let mut d := MaxDeque.empty
  let mut ok := true
  for s in stream do
    d := d.step s w
    let dv := d.front
    let nv := naiveMaxAt stream s.time w
    if dv != nv then
      IO.println s!"FAIL t={s.time}: deque={dv} naive={nv}"
      ok := false
  IO.println (if ok then "PASS" else "FAIL")

end Exec

#eval Exec.verify #[⟨1,5⟩, ⟨2,2⟩, ⟨3,7⟩, ⟨4,1⟩, ⟨5,3⟩, ⟨6,8⟩, ⟨7,4⟩] 3
#eval Exec.verify #[⟨1,10⟩, ⟨2,8⟩, ⟨3,6⟩, ⟨4,4⟩, ⟨5,2⟩] 2
#eval Exec.verify #[⟨1,2⟩, ⟨2,4⟩, ⟨3,6⟩, ⟨4,8⟩, ⟨5,10⟩] 2
#eval Exec.verify #[⟨1,5⟩, ⟨2,3⟩, ⟨3,1⟩, ⟨4,2⟩, ⟨5,4⟩] 100
#eval Exec.verifyMax #[⟨1,5⟩, ⟨2,2⟩, ⟨3,7⟩, ⟨4,1⟩, ⟨5,3⟩, ⟨6,8⟩, ⟨7,4⟩] 3
#eval Exec.verifyMax #[⟨1,10⟩, ⟨2,8⟩, ⟨3,6⟩, ⟨4,4⟩, ⟨5,2⟩] 2
#eval Exec.verifyMax #[⟨1,2⟩, ⟨2,4⟩, ⟨3,6⟩, ⟨4,8⟩, ⟨5,10⟩] 2
#eval Exec.verifyMax #[⟨1,5⟩, ⟨2,3⟩, ⟨3,1⟩, ⟨4,2⟩, ⟨5,4⟩] 100
#eval Exec.verify #[⟨10,42⟩] 1