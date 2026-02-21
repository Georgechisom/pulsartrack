//! PulsarTrack - Anomaly Detector (Soroban)
//! On-chain anomaly detection for ad campaign traffic on Stellar.

#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String,
};

#[contracttype]
#[derive(Clone)]
pub enum AnomalyType {
    SuddenTrafficSpike,
    UnusualGeoPattern,
    BotLikePattern,
    ClickFarming,
    InvalidTraffic,
    SuspiciousPublisher,
}

#[contracttype]
#[derive(Clone)]
pub enum AnomalySeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[contracttype]
#[derive(Clone)]
pub struct AnomalyReport {
    pub report_id: u64,
    pub campaign_id: u64,
    pub publisher: Option<Address>,
    pub anomaly_type: AnomalyType,
    pub severity: AnomalySeverity,
    pub description: String,
    pub metrics_snapshot: String,  // JSON snapshot of metrics at detection time
    pub auto_action_taken: bool,
    pub reported_at: u64,
    pub resolved: bool,
    pub resolved_at: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct TrafficBaseline {
    pub campaign_id: u64,
    pub avg_impressions_per_hour: u64,
    pub avg_clicks_per_hour: u64,
    pub spike_threshold_pct: u32,  // % increase to trigger alert
    pub last_updated: u64,
}

#[contracttype]
pub enum DataKey {
    Admin,
    OracleAddress,
    ReportCounter,
    SpikeThreshold,
    Report(u64),
    Baseline(u64),        // campaign_id
    FlaggedPublisher(Address),
}

#[contract]
pub struct AnomalyDetectorContract;

#[contractimpl]
impl AnomalyDetectorContract {
    pub fn initialize(env: Env, admin: Address, oracle: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::OracleAddress, &oracle);
        env.storage().instance().set(&DataKey::ReportCounter, &0u64);
        env.storage().instance().set(&DataKey::SpikeThreshold, &300u32); // 300% = 3x normal
    }

    pub fn set_baseline(
        env: Env,
        oracle: Address,
        campaign_id: u64,
        avg_impressions: u64,
        avg_clicks: u64,
        spike_threshold: u32,
    ) {
        oracle.require_auth();
        let stored_oracle: Address = env.storage().instance().get(&DataKey::OracleAddress).unwrap();
        if oracle != stored_oracle {
            panic!("unauthorized");
        }

        let baseline = TrafficBaseline {
            campaign_id,
            avg_impressions_per_hour: avg_impressions,
            avg_clicks_per_hour: avg_clicks,
            spike_threshold_pct: spike_threshold,
            last_updated: env.ledger().timestamp(),
        };

        env.storage().persistent().set(&DataKey::Baseline(campaign_id), &baseline);
    }

    pub fn report_anomaly(
        env: Env,
        oracle: Address,
        campaign_id: u64,
        publisher: Option<Address>,
        anomaly_type: AnomalyType,
        severity: AnomalySeverity,
        description: String,
        metrics_snapshot: String,
        auto_action: bool,
    ) -> u64 {
        oracle.require_auth();
        let stored_oracle: Address = env.storage().instance().get(&DataKey::OracleAddress).unwrap();
        if oracle != stored_oracle {
            panic!("unauthorized");
        }

        let counter: u64 = env.storage().instance().get(&DataKey::ReportCounter).unwrap_or(0);
        let report_id = counter + 1;

        // Auto-flag critical publisher anomalies
        if let Some(ref pub_addr) = publisher {
            match severity {
                AnomalySeverity::Critical => {
                    env.storage().persistent().set(&DataKey::FlaggedPublisher(pub_addr.clone()), &true);
                }
                _ => {}
            }
        }

        let report = AnomalyReport {
            report_id,
            campaign_id,
            publisher,
            anomaly_type,
            severity,
            description,
            metrics_snapshot,
            auto_action_taken: auto_action,
            reported_at: env.ledger().timestamp(),
            resolved: false,
            resolved_at: None,
        };

        env.storage().persistent().set(&DataKey::Report(report_id), &report);
        env.storage().instance().set(&DataKey::ReportCounter, &report_id);

        env.events().publish(
            (symbol_short!("anomaly"), symbol_short!("detected")),
            (report_id, campaign_id),
        );

        report_id
    }

    pub fn resolve_anomaly(env: Env, admin: Address, report_id: u64) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("unauthorized");
        }

        let mut report: AnomalyReport = env
            .storage()
            .persistent()
            .get(&DataKey::Report(report_id))
            .expect("report not found");

        report.resolved = true;
        report.resolved_at = Some(env.ledger().timestamp());
        env.storage().persistent().set(&DataKey::Report(report_id), &report);
    }

    pub fn get_report(env: Env, report_id: u64) -> Option<AnomalyReport> {
        env.storage().persistent().get(&DataKey::Report(report_id))
    }

    pub fn get_baseline(env: Env, campaign_id: u64) -> Option<TrafficBaseline> {
        env.storage().persistent().get(&DataKey::Baseline(campaign_id))
    }

    pub fn is_publisher_flagged(env: Env, publisher: Address) -> bool {
        env.storage().persistent().get(&DataKey::FlaggedPublisher(publisher)).unwrap_or(false)
    }

    pub fn get_report_count(env: Env) -> u64 {
        env.storage().instance().get(&DataKey::ReportCounter).unwrap_or(0)
    }
}

mod test;
