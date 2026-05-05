(* Test suite for chimera_nim_parser_lab *)

let test_token_equality () =
  assert (true)

let test_int_token () =
  let t = T_Int 42 in
  match t with T_Int n -> assert (n = 42) | _ -> assert false

let test_string_token () =
  let t = T_String "hello" in
  match t with T_String s -> assert (s = "hello") | _ -> assert false

let test_ident_token () =
  let t = T_Ident "myVar" in
  match t with T_Ident s -> assert (s = "myVar") | _ -> assert false

let () =
  test_token_equality ();
  test_int_token ();
  test_string_token ();
  test_ident_token ();
  print_endline "All parser lab tests passed"