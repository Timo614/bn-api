use crate::functional::base;
use db::prelude::*;

#[cfg(test)]
mod create_tests {
    use super::*;
    #[actix_rt::test]
    async fn create_admin() {
        base::collections::create(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn create_super() {
        base::collections::create(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn create_user() {
        base::collections::create(Roles::User, true).await;
    }
}

#[cfg(test)]
mod index {
    use super::*;
    #[actix_rt::test]
    async fn index_admin() {
        base::collections::index(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn index_super() {
        base::collections::index(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn index_user() {
        base::collections::index(Roles::User, true).await;
    }
}

#[cfg(test)]
mod update {
    use super::*;
    #[actix_rt::test]
    async fn update_admin() {
        base::collections::update(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn update_super() {
        base::collections::update(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn update_user() {
        base::collections::update(Roles::User, true).await;
    }
}

#[cfg(test)]
mod delete {
    use super::*;
    #[actix_rt::test]
    async fn delete_admin() {
        base::collections::delete(Roles::Admin, true).await;
    }
    #[actix_rt::test]
    async fn delete_super() {
        base::collections::delete(Roles::Super, true).await;
    }
    #[actix_rt::test]
    async fn delete_user() {
        base::collections::delete(Roles::User, true).await;
    }
}
