#[derive(Debug, Clone)]
pub struct Ec2InstanceInfo {
    pub id: String,
    pub name: Option<String>,
    pub instance_type: String,
    pub state: String,
    pub az: String,
    pub private_ip: Option<String>,
}
