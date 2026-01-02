# tfcost (Rust)

Terraform plan → AWS daily/monthly cost estimator with an optional TUI.

## Quick start

```bash
# 1) Create JSON plan output from Terraform:
terraform plan -out=plan.out
terraform show -json plan.out > plan.json

# 2) Run the estimator (plain table):
cargo run -- --plan plan.json --region us-east-1

# 3) TUI mode:
cargo run -- --plan plan.json --region us-east-1 --tui
