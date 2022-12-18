#[derive(Clone)]
pub enum Trustee {
    CurrentUser,
    Name(String),
}
