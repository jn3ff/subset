// use subset::Subset;

// struct Cube {
//     side: usize,
// }

// #[derive(Subset)]
// #[subset(from = "Cube", as_ref)]
// struct Square<'a> {
//     side: &'a usize,
// }

// #[test]
// fn converts_cube_into_square() {
//     let cube = Cube { side: 6 };
//     let square: Square = cube.into();
//     assert_eq!(square.side, 6);
// }
