#[macro_use]
extern crate nom;

extern crate string_cache;

pub mod db;
pub mod metric;

/// How metrics come into the agent.
pub mod recv;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
