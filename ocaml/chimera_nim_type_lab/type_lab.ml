(* Type inference prototype for Chimera-Nim *)

(* Simple type representation *)
type ty =
  | TInt
  | TFloat
  | TString
  | TBool
  | TUnit
  | TSeq of ty
  | TFun of ty list * ty
  | TVar of string

(* Type constraint solver - simple unification *)
let rec unify t1 t2 =
  match (t1, t2) with
  | (TInt, TInt) -> Some []
  | (TFloat, TFloat) -> Some []
  | (TString, TString) -> Some []
  | (TBool, TBool) -> Some []
  | (TUnit, TUnit) -> Some []
  | (TVar v, t) -> Some [(v, t)]
  | (t, TVar v) -> Some [(v, t)]
  | (TSeq a, TSeq b) -> unify a b
  | (TFun (args1, ret1), TFun (args2, ret2)) ->
     (match unify_list args1 args2 with
      | Some cs -> (match unify ret1 ret2 with
                    | Some cs2 -> Some (cs @ cs2)
                    | None -> None)
      | None -> None)
  | _ -> None

and unify_list ts1 ts2 =
  match (ts1, ts2) with
  | ([], []) -> Some []
  | (t1 :: rest1, t2 :: rest2) ->
     (match unify t1 t2 with
      | Some cs1 ->
         (match unify_list rest1 rest2 with
          | Some cs2 -> Some (cs1 @ cs2)
          | None -> None)
      | None -> None)
  | _ -> None

(* Test the type inference *)
let () =
  match unify TInt TInt with
  | Some _ -> print_endline "Type inference test passed"
  | None -> print_endline "Type inference test failed"