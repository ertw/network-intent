forbid(principal, action, resource)
when { principal.role == "witness" &&
  (action == Action::"AssignApply" || action == Action::"SubmitApplyReceipt" ||
   action == Action::"EnrollAgent" || action == Action::"RevokeAgent") };

forbid(principal, action, resource)
when { principal.role == "apply" &&
  (action == Action::"AssignWitness" || action == Action::"AssignApply" ||
   action == Action::"SubmitEvidence" || action == Action::"EnrollAgent" ||
   action == Action::"RevokeAgent") };

permit(principal, action == Action::"ReadState", resource)
when { principal.role == "controller" || principal.role == "witness" || principal.role == "apply" };

permit(principal, action == Action::"SubmitEvidence", resource)
when { principal.role == "witness" };

permit(principal, action == Action::"SubmitApplyReceipt", resource)
when { principal.role == "apply" && principal.device_id == resource.device_id };

permit(principal, action == Action::"AssignWitness", resource)
when { principal.role == "controller" };

permit(principal, action == Action::"AssignApply", resource)
when { principal.role == "controller" };

permit(principal, action == Action::"EnrollAgent", resource)
when { principal.role == "controller" };

permit(principal, action == Action::"RevokeAgent", resource)
when { principal.role == "controller" };
