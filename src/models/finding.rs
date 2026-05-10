#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    High,
    Medium,
}

impl FindingSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingSeverity::High => "HIGH",
            FindingSeverity::Medium => "MED",
        }
    }

    pub fn rank(&self) -> u8 {
        match self {
            FindingSeverity::High => 0,
            FindingSeverity::Medium => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCategory {
    Incident,
    Waste,
    Hygiene,
}

impl FindingCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            FindingCategory::Incident => "Incident",
            FindingCategory::Waste => "Waste",
            FindingCategory::Hygiene => "Hygiene",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingRoute {
    Ec2,
    CloudWatch,
    Lambda,
    Rds,
    Secrets,
    Sqs,
    TargetGroups,
    SecurityGroups,
    Vpc,
}

#[derive(Debug, Clone)]
pub struct Finding {
    pub severity: FindingSeverity,
    pub category: FindingCategory,
    pub service: String,
    pub region: String,
    pub summary: String,
    pub next_step: String,
    pub route: FindingRoute,
}
