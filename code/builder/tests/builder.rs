use builder::Build;

enum

#[derive(Default, Build)]
struct Request {
    #[required]
    url: String,

    path: Option<String>
}


#[test]
fn test_builder() {
    let request = Request::new("example.com")
        .path("tester")
}