#[cfg(test)]
mod tests {
    use crate::CustomersRepository;
    use ormcle_macro::Table;

    #[derive(Table, Debug)]
    pub struct Customers {
        #[id(0)]
        customer_id: i32,
        email_address: Option<String>,
        full_name: String,
    }

    #[test]
    fn connect_to_db() {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let oracle = sibyl::Environment::new().unwrap();
                    let session = oracle
                        .connect("127.0.0.1:1521/ORCLPDB1", "PDBADMIN", "ORACLE")
                        .await
                        .unwrap();

                    session.ping().await.unwrap();
                });
            });
        });
    }

    #[test]
    fn non_existing_id_returns_none() {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let oracle = sibyl::Environment::new().unwrap();
                    let session = oracle
                        .connect("127.0.0.1:1521/ORCLPDB1", "PDBADMIN", "ORACLE")
                        .await
                        .unwrap();
                    let repo = CustomersRepository::new(&session);
                    assert!(repo.find_by_id(0).await.is_ok_and(|r| r.is_none()))
                });
            });
        });
    }

    #[test]
    fn query_single_successfully() {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let oracle = sibyl::Environment::new().unwrap();
                    let session = oracle
                        .connect("127.0.0.1:1521/ORCLPDB1", "PDBADMIN", "ORACLE")
                        .await
                        .unwrap();
                    let repo = CustomersRepository::new(&session);
                    let result = repo.find_by_id(1).await;
                    dbg!(&result);
                    assert!(result.is_ok_and(|res| res.is_some()))
                });
            });
        });
    }

    #[test]
    fn query_multiple_successfuly() {
        std::thread::scope(|scope| {
            scope.spawn(|| {
                let rt = tokio::runtime::Runtime::new().unwrap();
                rt.block_on(async {
                    let oracle = sibyl::Environment::new().unwrap();
                    let session = oracle
                        .connect("127.0.0.1:1521/ORCLPDB1", "PDBADMIN", "ORACLE")
                        .await
                        .unwrap();
                    let repo = CustomersRepository::new(&session);
                    let result = repo.find_all().await;
                    assert!(result.is_ok())
                });
            });
        });
    }
}
