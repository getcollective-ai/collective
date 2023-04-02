use derive_discriminant::Discriminant;

#[derive(Discriminant)]
#[derive(Debug, PartialEq, Eq)]
enum Abc {
    A,
    B { x: usize, y: usize },
}

#[test]
fn test_abc_to_discriminant() {
    let a = A::try_from(Abc::A).unwrap();
    assert_eq!(a, A);
    let not_a = A::try_from(Abc::B { x: 1, y: 2 });
    assert!(not_a.is_err());

    let b = B::try_from(Abc::B { x: 1, y: 2 }).unwrap();
    assert_eq!(b, B { x: 1, y: 2 });
    let not_b = B::try_from(Abc::A);
    assert!(not_b.is_err());
}

#[test]
fn test_discriminant_to_abc() {
    let a = Abc::from(A);
    assert_eq!(a, Abc::A);
    let b = Abc::from(B { x: 1, y: 2 });
    assert_eq!(b, Abc::B { x: 1, y: 2 });
}
