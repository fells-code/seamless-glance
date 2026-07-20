use crate::models::ec2::Ec2InstanceInfo;

pub struct SshContext {
    pub instance_name: String,
    pub user: String,
    pub host: String,
    pub key_name: Option<String>,
}

pub fn ssh_command(instance: &Ec2InstanceInfo) -> Option<SshContext> {
    Some(SshContext {
        instance_name: instance.label(),
        user: "ec2-user".into(),
        host: instance.public_ip.as_ref()?.clone(),
        key_name: instance.key_name.clone(),
    })
}
