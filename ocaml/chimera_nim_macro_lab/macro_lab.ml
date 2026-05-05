(* Macro expansion prototype for Chimera-Nim *)

(* Simple AST representation for macro expansion *)
type ast =
  | IntLit of int
  | StringLit of string
  | Ident of string
  | BinOp of string * ast * ast
  | Call of string * ast list
  | Block of ast list

(* Template expansion - simple substitution *)
let rec expand_template template params =
  match template with
  | IntLit n -> IntLit n
  | StringLit s -> StringLit s
  | Ident "x" when List.length params > 0 ->
     (match List.hd params with
      | Some p -> p
      | None -> Ident "x")
  | Ident s -> Ident s
  | BinOp (op, e1, e2) ->
     BinOp (op, expand_template e1 params, expand_template e2 params)
  | Call (name, args) ->
     Call (name, List.map (fun a -> expand_template a params) args)
  | Block stmts ->
     Block (List.map (fun s -> expand_template s params) [])

(* Test macro expansion *)
let () =
  let template = BinOp ("+", Ident "x", IntLit 1) in
  let result = expand_template template [Some (IntLit 5)] in
  match result with
  | BinOp ("+", IntLit 5, IntLit 1) -> print_endline "Macro expansion test passed"
  | _ -> print_endline "Macro expansion test failed"