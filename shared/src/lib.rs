pub trait OrmcleId {
    fn fields() -> Option<&'static [&'static str]>;
}

impl OrmcleId for i32 {
    fn fields() -> Option<&'static [&'static str]> {
        None
    }
}

impl OrmcleId for i16 {
    fn fields() -> Option<&'static [&'static str]> {
        None
    }
}

impl OrmcleId for i8 {
    fn fields() -> Option<&'static [&'static str]> {
        None
    }
}

struct Id {
    first: i32,
    sec: i16,
}

impl OrmcleId for Id {
    fn fields() -> Option<&'static [&'static str]> {
        Some(["first", "second"].as_slice())
    }
}

struct Struct {
    id: Id,
}

struct Prim {
    id: i32,
}

impl Struct {}
