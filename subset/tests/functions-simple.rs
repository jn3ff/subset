#![cfg(feature = "functions")]
use subset::Subset;

#[derive(Clone)]
struct Cube {
    side: usize,
}

impl Cube {
    fn scale(&mut self, scalar: usize) {
        self.side *= scalar;
    }
}

#[derive(Subset)]
#[subset(from = "Cube")]
#[subset(functions = "from::scale")]
struct Square {
    side: usize,
}

#[test]
fn scale_works_on_both() {
    let mut cube = Cube { side: 6 };
    let mut square: Square = cube.clone().into();
    cube.scale(2);
    square.scale(2);
    assert_eq!(cube.side, 12);
    assert_eq!(square.side, 12);
}
