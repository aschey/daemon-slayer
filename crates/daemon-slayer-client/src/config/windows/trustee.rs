#[derive(Clone, Debug)]
pub enum Trustee {
    CurrentUser,
    Name(String),
}
