(* Token types for Nim lexer *)
type token =
  | T_Int of int
  | T_Float of float
  | T_String of string
  | T_Ident of string
  | T_Keyword of string
  | T_Symbol of string
  | T_Eof

(* A simple token for testing *)
let test_token () =
  T_Int 42

(* Test token equality *)
let token_eq t1 t2 =
  match (t1, t2) with
  | (T_Int n1, T_Int n2) -> n1 = n2
  | (T_String s1, T_String s2) -> s1 = s2
  | (T_Ident s1, T_Ident s2) -> s1 = s2
  | _ -> false

(* Test that we can create tokens *)
let () =
  let t = test_token () in
  assert (token_eq t (T_Int 42));
  print_endline "Parser lab tests passed"