use error_stack::IntoReport;

use super::{MockDb, Store};
use crate::{
    connection,
    core::errors::{self, CustomResult},
    types::storage::{self, DisputeDbExt},
};
use crate::types::transformers::ForeignInto;

#[async_trait::async_trait]
pub trait DisputeInterface {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;

    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError>;

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &str,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;

    async fn find_disputes_by_merchant_id(
        &self,
        merchant_id: &str,
        dispute_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError>;

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError>;

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError>;
}

#[async_trait::async_trait]
impl DisputeInterface for Store {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        dispute
            .insert(&conn)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_payment_id_connector_dispute_id(
            &conn,
            merchant_id,
            payment_id,
            connector_dispute_id,
        )
        .await
        .map_err(Into::into)
        .into_report()
    }

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &str,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_dispute_id(&conn, merchant_id, dispute_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_disputes_by_merchant_id(
        &self,
        merchant_id: &str,
        dispute_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::filter_by_constraints(&conn, merchant_id, dispute_constraints)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let conn = connection::pg_connection_read(self).await?;
        storage::Dispute::find_by_merchant_id_payment_id(&conn, merchant_id, payment_id)
            .await
            .map_err(Into::into)
            .into_report()
    }

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let conn = connection::pg_connection_write(self).await?;
        this.update(&conn, dispute)
            .await
            .map_err(Into::into)
            .into_report()
    }
}

#[async_trait::async_trait]
impl DisputeInterface for MockDb {
    async fn insert_dispute(
        &self,
        dispute: storage::DisputeNew,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let mut locked_disputes = self.disputes.lock().await;

        if locked_disputes
            .iter()
            .any(|d| d.dispute_id == dispute.dispute_id)
        {
            Err(errors::StorageError::MockDbError)?;
        }
        let now = common_utils::date_time::now();

        let new_dispute = storage::Dispute {
            id: locked_disputes.len() as i32,
            dispute_id: dispute.dispute_id,
            amount: dispute.amount,
            currency: dispute.currency,
            dispute_stage: dispute.dispute_stage,
            dispute_status: dispute.dispute_status,
            payment_id: dispute.payment_id,
            attempt_id: dispute.attempt_id,
            merchant_id: dispute.merchant_id,
            connector_status: dispute.connector_status,
            connector_dispute_id: dispute.connector_dispute_id,
            connector_reason: dispute.connector_reason,
            connector_reason_code: dispute.connector_reason_code,
            challenge_required_by: dispute.challenge_required_by,
            connector_created_at: dispute.connector_created_at,
            connector_updated_at: dispute.connector_updated_at,
            created_at: now,
            modified_at: now,
            connector: dispute.connector,
            evidence: dispute.evidence.unwrap(),
        };

        locked_disputes.push(new_dispute.clone());

        Ok(new_dispute)
    }
    async fn find_by_merchant_id_payment_id_connector_dispute_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
        connector_dispute_id: &str,
    ) -> CustomResult<Option<storage::Dispute>, errors::StorageError> {
        Ok(self
            .disputes
            .lock()
            .await
            .iter()
            .find(|d| {
                d.merchant_id == merchant_id
                    && d.payment_id == payment_id
                    && d.connector_dispute_id == connector_dispute_id
            })
            .cloned())
    }

    async fn find_dispute_by_merchant_id_dispute_id(
        &self,
        merchant_id: &str,
        dispute_id: &str,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        let found_dispute = locked_disputes
            .iter()
            .find(|d| d.merchant_id == merchant_id && d.dispute_id == dispute_id)
            .cloned();

        if let Some(dispute) = found_dispute {
            Ok(dispute)
        } else{
            Err(errors::StorageError::ValueNotFound(format!("No dispute available for merchant_id = {merchant_id} and dispute_id = {dispute_id}")))?
        }
    }

    async fn find_disputes_by_merchant_id(
        &self,
        merchant_id: &str,
        dispute_constraints: api_models::disputes::DisputeListConstraints,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;
/*
        Ok(locked_disputes
            .iter()
            .filter(|d| {
                let mut filtering_condition = d.merchant_id == merchant_id;

                if let Some(dispute_status) = dispute_constraints.dispute_status {
                    filtering_condition &= d.dispute_status == dispute_status.foreign_into()
                }

                if let Some(dispute_stage) = dispute_constraints.dispute_stage {
                    filtering_condition &= d.dispute_stage == dispute_stage.foreign_into()
                }

                if d.connector_reason.is_some() {
                    if let Some(reason) = dispute_constraints.reason {
                        filtering_condition &= d.connector_reason.unwrap() == reason
                    }
                }

                if let Some(connector) = dispute_constraints.connector {
                    filtering_condition &= d.connector == connector
                }

                if let Some(received_time) = dispute_constraints.received_time {
                    filtering_condition &= d.created_at == received_time
                }

                if let Some(received_time_lt) = dispute_constraints.received_time_lt {
                    filtering_condition &= d.created_at < received_time_lt
                }

                if let Some(received_time_gt) = dispute_constraints.received_time_gt {
                    filtering_condition &= d.created_at > received_time_gt
                }

                if let Some(received_time_lte) = dispute_constraints.received_time_lte {
                    filtering_condition &= d.created_at <= received_time_lte
                }

                if let Some(received_time_gte) = dispute_constraints.received_time_gte {
                    filtering_condition &= d.created_at >= received_time_gte
                }

                filtering_condition
            })
            .take(if let Some(limit) = dispute_constraints.limit {
                limit as usize
            } else {
                usize::MAX
            })
            .cloned()
            .collect())

 */
        Ok(vec![])
    }

    async fn find_disputes_by_merchant_id_payment_id(
        &self,
        merchant_id: &str,
        payment_id: &str,
    ) -> CustomResult<Vec<storage::Dispute>, errors::StorageError> {
        let locked_disputes = self.disputes.lock().await;

        Ok(locked_disputes
            .iter()
            .filter(|d| d.merchant_id == merchant_id && d.payment_id == payment_id)
            .cloned()
            .collect())
    }

    async fn update_dispute(
        &self,
        this: storage::Dispute,
        dispute: storage::DisputeUpdate,
    ) -> CustomResult<storage::Dispute, errors::StorageError> {
        let mut locked_disputes = self.disputes.lock().await;

        let mut dispute_to_update = locked_disputes
            .iter_mut()
            .find(|d| d.dispute_id == this.dispute_id)
            .ok_or(errors::StorageError::MockDbError)?;

        let now = common_utils::date_time::now();

        match dispute {
            storage::DisputeUpdate::Update {
                dispute_stage,
                dispute_status,
                connector_status,
                connector_reason,
                connector_reason_code,
                challenge_required_by,
                connector_updated_at,
            } => {
                if connector_reason.is_some() {
                    dispute_to_update.connector_reason = connector_reason;
                }

                if connector_reason_code.is_some() {
                    dispute_to_update.connector_reason_code = connector_reason_code;
                }

                if challenge_required_by.is_some() {
                    dispute_to_update.challenge_required_by = challenge_required_by;
                }

                if connector_updated_at.is_some() {
                    dispute_to_update.connector_updated_at = connector_updated_at;
                }

                dispute_to_update.dispute_stage = dispute_stage;
                dispute_to_update.dispute_status = dispute_status;
                dispute_to_update.connector_status = connector_status;
            }
            storage::DisputeUpdate::StatusUpdate {
                dispute_status,
                connector_status,
            } => {
                if let Some(status) = connector_status {
                    dispute_to_update.connector_status = status;
                }
                dispute_to_update.dispute_status = dispute_status;
            }
            storage::DisputeUpdate::EvidenceUpdate { evidence } => {
                dispute_to_update.evidence = evidence;
            }
        }

        dispute_to_update.modified_at = now;

        Ok(dispute_to_update.clone())
    }
}

#[cfg(test)]
mod tests {
    mod mockdb_dispute_interface {
        use std::str::FromStr;
        use serde_json::Value;
        use time::macros::datetime;
        use time::PrimitiveDateTime;
        use masking::Secret;
        use storage_models::dispute::Dispute;
        use storage_models::enums::{DisputeStage, DisputeStatus};

        use crate::{
            db::{dispute::DisputeInterface, MockDb},
            types::storage,
        };

        fn disputes_eq(d1: Dispute, d2: Dispute) {
            assert_eq!(d1.id, d2.id);
            assert_eq!(d1.dispute_id, d2.dispute_id);
            assert_eq!(d1.amount, d2.amount);
            assert_eq!(d1.currency, d2.currency);
            assert_eq!(d1.dispute_stage, d2.dispute_stage);
            assert_eq!(d1.dispute_status, d2.dispute_status);
            assert_eq!(d1.payment_id, d2.payment_id);
            assert_eq!(d1.attempt_id, d2.attempt_id);
            assert_eq!(d1.merchant_id, d2.merchant_id);
            assert_eq!(d1.connector_status, d2.connector_status);
            assert_eq!(d1.connector_dispute_id, d2.connector_dispute_id);
            assert_eq!(d1.connector_reason, d2.connector_reason);
            assert_eq!(d1.connector_reason_code, d2.connector_reason_code);
            assert_eq!(d1.challenge_required_by, d2.challenge_required_by);
            assert_eq!(d1.connector_created_at, d2.connector_created_at);
            assert_eq!(d1.connector_updated_at, d2.connector_updated_at);
            assert_eq!(d1.created_at, d2.created_at);
            assert_eq!(d1.modified_at, d2.modified_at);
            assert_eq!(d1.connector, d2.connector);
            assert_eq!(d1.evidence, d2.evidence);
        }

        #[allow(clippy::unwrap_used)]
        #[tokio::test]
        async fn test_insert_dispute() {
            let mockdb = MockDb::new(&Default::default()).await;

            let created_dispute = mockdb
                .insert_dispute(storage::DisputeNew {
                    dispute_id: "dispute_id".into(),
                    amount: "amount".into(),
                    currency: "currency".into(),
                    dispute_stage: DisputeStage::Dispute,
                    dispute_status: DisputeStatus::DisputeOpened,
                    payment_id: "payment_id".into(),
                    attempt_id: "attempt_id".into(),
                    merchant_id: "merchant_id".into(),
                    connector_status: "connector_status".into(),
                    connector_dispute_id: "connector_dispute_id".into(),
                    connector_reason: Some("connector_reason".into()),
                    connector_reason_code: Some("connector_reason_code".into()),
                    challenge_required_by: Some(datetime!(2019-01-01 0:00)),
                    connector_created_at: Some(datetime!(2019-01-02 0:00)),
                    connector_updated_at: Some(datetime!(2019-01-03 0:00)),
                    connector: "connector".into(),
                    evidence: Some(Secret::from(Value::String("evidence".into()))),
                })
                .await
                .unwrap();

            let found_dispute = mockdb.disputes.lock().await.iter().find(|d| d.dispute_id == created_dispute.dispute_id).cloned();

            assert!(found_dispute.is_some());

            disputes_eq(created_dispute, found_dispute.unwrap());
        }

        #[allow(clippy::unwrap_used)]
        //#[tokio::test]
        async fn test_find_by_merchant_id_payment_id_connector_dispute_id() {
            let mockdb = MockDb::new(&Default::default()).await;

        }

        #[allow(clippy::unwrap_used)]
        //#[tokio::test]
        async fn test_find_dispute_by_merchant_id_dispute_id() {
            let mockdb = MockDb::new(&Default::default()).await;

        }

        #[allow(clippy::unwrap_used)]
        //#[tokio::test]
        async fn test_find_disputes_by_merchant_id() {
            let mockdb = MockDb::new(&Default::default()).await;

        }

        #[allow(clippy::unwrap_used)]
        //#[tokio::test]
        async fn test_find_disputes_by_merchant_id_payment_id() {
            let mockdb = MockDb::new(&Default::default()).await;

        }

        #[allow(clippy::unwrap_used)]
        //#[tokio::test]
        async fn test_update_dispute() {
            let mockdb = MockDb::new(&Default::default()).await;

        }
    }
}