---
name: aws-posture
description: Read live AWS posture strictly read-only and compare it to the IaC
allowed-tools: table, write_file
---
Look at what is actually deployed, not just what the Terraform says.

**The hard rule: read-only. Always.** Use only `describe`, `list`, `get`, and `head`
operations. Never `create`, `update`, `delete`, `put`, `attach`, `detach`, or `apply`. If a
check appears to need a write, it does not get done — stop and say so. You are looking at
someone's production account, and there is no read that justifies a write.

1. **Confirm which account and region you are pointed at** before anything else, and say so
   out loud: `aws sts get-caller-identity`. A review of the wrong account is worse than no
   review, because it comes back clean.
2. **Sweep the exposures that actually get people breached.** Write each command's raw
   output to `outputs/aws-posture.json` as you go, so the evidence behind every claim is
   re-readable and you are never quoting from memory:
   - S3: public access blocks, bucket policies, ACLs, default encryption.
   - Security groups: ingress from `0.0.0.0/0`, and to which ports.
   - IAM: wildcard actions and resources, unused long-lived access keys, users without MFA.
   - RDS and EBS: public accessibility, encryption at rest.
   - CloudTrail: is it on, in every region, and going somewhere durable?
3. **Compare live state to the IaC.** Drift is a finding in itself: something changed outside
   Terraform, which means it will change back on the next apply, or it means someone is
   working around review. Say which resources drifted and in which direction.
4. **Never print credentials or account identifiers that embed them.** Refer to resources by
   name and type.
5. **Fix in the Terraform**, never in the console — see the rule at the top. If a resource
   has no IaC at all, say so: adopting it into Terraform is the fix, and it is a decision
   the user makes.
6. **Deliver:** findings table (severity · resource · what's exposed · fix), the drift list,
   and the IaC branch.
