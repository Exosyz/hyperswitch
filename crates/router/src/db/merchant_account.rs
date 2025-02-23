use common_utils::ext_traits::AsyncExt;
use error_stack::{IntoReport, ResultExt};

use super::{MockDb, Store};
#[cfg(feature = "accounts_cache")]
use crate::cache::{self, ACCOUNTS_CACHE};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    db::MasterKeyInterface,
    types::{
        domain::{
            self,
            behaviour::{Conversion, ReverseConversion},
        },
        storage,
    },
};

#[async_trait::async_trait]
pub trait MerchantAccountInterface
where
    domain::MerchantAccount:
        Conversion<DstType = storage::MerchantAccount, NewDstType = storage::MerchantAccountNew>,
{
    async fn insert_merchant(
        &self,
        merchant_account: domain::MerchantAccount,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn update_merchant(
        &self,
        this: domain::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &str,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError>;

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError>;
}

#[async_trait::async_trait]
impl MerchantAccountInterface for Store {
    async fn insert_merchant(
        &self,
        merchant_account: domain::MerchantAccount,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        let merchant_id = merchant_account.merchant_id.clone();
        merchant_account
            .construct_new()
            .await
            .change_context(errors::StorageError::EncryptionError)?
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()?
            .convert(self, &merchant_id, self.get_migration_timestamp())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let fetch_func = || async {
            let conn = connection::pg_connection_read(self).await?;
            storage::MerchantAccount::find_by_merchant_id(&conn, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            fetch_func()
                .await?
                .convert(self, merchant_id, self.get_migration_timestamp())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::get_or_populate_in_memory(self, merchant_id, fetch_func, &ACCOUNTS_CACHE)
                .await?
                .convert(self, merchant_id, self.get_migration_timestamp())
                .await
                .change_context(errors::StorageError::DecryptionError)
        }
    }

    async fn update_merchant(
        &self,
        this: domain::MerchantAccount,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let _merchant_id = this.merchant_id.clone();
        let update_func = || async {
            let conn = connection::pg_connection_write(self).await?;
            Conversion::convert(this)
                .await
                .change_context(errors::StorageError::EncryptionError)?
                .update(&conn, merchant_account.into())
                .await
                .map_err(Into::into)
                .into_report()
                .async_and_then(|item| async {
                    item.convert(self, &_merchant_id, self.get_migration_timestamp())
                        .await
                        .change_context(errors::StorageError::DecryptionError)
                })
                .await
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            update_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::publish_and_redact(
                self,
                cache::CacheKind::Accounts((&_merchant_id).into()),
                update_func,
            )
            .await
        }
    }

    async fn update_specific_fields_in_merchant(
        &self,
        merchant_id: &str,
        merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let update_func = || async {
            let conn = connection::pg_connection_write(self).await?;
            storage::MerchantAccount::update_with_specific_fields(
                &conn,
                merchant_id,
                merchant_account.into(),
            )
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|item| async {
                item.convert(self, merchant_id, self.get_migration_timestamp())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            update_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::publish_and_redact(
                self,
                cache::CacheKind::Accounts(merchant_id.into()),
                update_func,
            )
            .await
        }
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        publishable_key: &str,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::MerchantAccount::find_by_publishable_key(&conn, publishable_key)
            .await
            .map_err(Into::into)
            .into_report()
            .async_and_then(|item| async {
                let merchant_id = item.merchant_id.clone();
                item.convert(self, &merchant_id, self.get_migration_timestamp())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        let delete_func = || async {
            let conn = connection::pg_connection_write(self).await?;
            storage::MerchantAccount::delete_by_merchant_id(&conn, merchant_id)
                .await
                .map_err(Into::into)
                .into_report()
        };

        #[cfg(not(feature = "accounts_cache"))]
        {
            delete_func().await
        }

        #[cfg(feature = "accounts_cache")]
        {
            super::cache::publish_and_redact(
                self,
                cache::CacheKind::Accounts(merchant_id.into()),
                delete_func,
            )
            .await
        }
    }
}

#[async_trait::async_trait]
impl MerchantAccountInterface for MockDb {
    #[allow(clippy::panic)]
    async fn insert_merchant(
        &self,
        merchant_account: domain::MerchantAccount,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let mut accounts = self.merchant_accounts.lock().await;
        let account = Conversion::convert(merchant_account)
            .await
            .change_context(errors::StorageError::EncryptionError)?;
        let merchant_id = account.merchant_id.clone();
        accounts.push(account.clone());

        account
            .convert(self, &merchant_id, self.get_migration_timestamp())
            .await
            .change_context(errors::StorageError::DecryptionError)
    }

    #[allow(clippy::panic)]
    async fn find_merchant_account_by_merchant_id(
        &self,
        merchant_id: &str,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        let accounts = self.merchant_accounts.lock().await;
        let account: Option<domain::MerchantAccount> = accounts
            .iter()
            .find(|account| account.merchant_id == merchant_id)
            .cloned()
            .async_map(|a| async {
                a.convert(self, merchant_id, self.get_migration_timestamp())
                    .await
                    .change_context(errors::StorageError::DecryptionError)
            })
            .await
            .transpose()?;

        match account {
            Some(account) => Ok(account),
            // [#172]: Implement function for `MockDb`
            None => Err(errors::StorageError::MockDbError)?,
        }
    }

    async fn update_merchant(
        &self,
        _this: domain::MerchantAccount,
        _merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn update_specific_fields_in_merchant(
        &self,
        _merchant_id: &str,
        _merchant_account: storage::MerchantAccountUpdate,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        // [#TODO]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn find_merchant_account_by_publishable_key(
        &self,
        _publishable_key: &str,
    ) -> CustomResult<domain::MerchantAccount, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }

    async fn delete_merchant_account_by_merchant_id(
        &self,
        _merchant_id: &str,
    ) -> CustomResult<bool, errors::StorageError> {
        // [#172]: Implement function for `MockDb`
        Err(errors::StorageError::MockDbError)?
    }
}
