use ormcle_macro::{Id, Table, Test};

#[derive(Table, Debug)]
#[ormcle(table_name = "CUSTOMERS")]
pub struct CustomersStruct {
    #[ormcle(primary_key, column_name = "CUSTOMER_ID")]
    id: i32,
    #[ormcle(column_name = "EMAIL_ADDRESS")]
    email: Option<String>,
    #[ormcle(column_name = "FULL_NAME")]
    name: String,
}

#[derive(Debug, Test)]
struct OrderId {
    order_id: i32,
    line_item_id: i32,
}

// #[derive(Debug, Table)]
// struct Order {
//     // #[ormcle(primary_key)]
//     // id: OrderId,
//     product_id: i32,
//     unit_price: f32,
//     quantity: f32,
//     shipment_id: i32,
// }

#[cfg(test)]
mod tests {
    use crate::CustomersStructRepository;

    #[test]
    fn find_all_has_data() {
        let oracle = sibyl::Environment::new().unwrap();
        let session = oracle
            .connect("127.0.0.1:1521/ORCLPDB1", "PDBADMIN", "ORACLE") // just a generic test db, okay to leak
            .unwrap();
        let repo = CustomersStructRepository { session };
        let result = repo.find_all().unwrap();
        assert!(!result.is_empty())
    }

    #[test]
    fn find_by_id_has_data_if_valid_id() {
        let oracle = sibyl::Environment::new().unwrap();
        let session = oracle
            .connect("127.0.0.1:1521/ORCLPDB1", "PDBADMIN", "ORACLE") // just a generic test db, okay to leak
            .unwrap();
        let repo = CustomersStructRepository { session };
        let result = repo.find_by_id(1);
        assert!(result.is_ok_and(|r| r.is_some()))
    }

    #[test]
    fn find_by_id_returns_none_if_not_found() {
        let oracle = sibyl::Environment::new().unwrap();
        let session = oracle
            .connect("127.0.0.1:1521/ORCLPDB1", "PDBADMIN", "ORACLE") // just a generic test db, okay to leak
            .unwrap();
        let repo = CustomersStructRepository { session };
        let result = repo.find_by_id(0);
        assert!(result.is_ok_and(|r| r.is_none()))
    }
}
