//! The controller's only revision promotion boundary. Source is interpreted by
//! the installed Idris compiler, never by a client-supplied success verdict.
use intent_protocol::{payload_digest, RevisionRef, SourceSpan, assurance::*, check::{check_plan, CoverageContext}};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, HashSet}, path::PathBuf, process::Stdio, time::Duration};
use thiserror::Error;
use tokio::{io::AsyncReadExt, process::Command, time::timeout};

const MAX_SOURCE: usize = 1_048_576;
const MAX_OUTPUT: usize = 16 * 1_048_576;

#[derive(Debug, Error)]
pub enum AdmissionError {
    #[error("compiler I/O: {0}")] Io(#[from] std::io::Error),
    #[error("compiler invocation exceeded resource limits")] Limits,
    #[error("invalid compiler output: {0}")] Encoding(#[from] serde_json::Error),
    #[error("compiler rejected source: {0}")] InvalidSource(String),
    #[error("assurance coverage blocked: {0}")] Coverage(String),
    #[error("revision or plan binding differs from authoritative compiler result")] Binding,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerWitness {
    pub version: u32,
    pub target: String,
    pub profile: String,
    pub claims: Vec<CompilerClaim>,
    #[serde(rename = "coverageBlockers")]
    pub coverage_blockers: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompilerClaim {
    pub id: String,
    pub kind: String,
    pub target: String,
    pub source: CompilerSpan,
    pub bindings: Vec<FieldBinding>,
    pub unsupported: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompilerSpan { pub file: String, pub line: u32, pub column: u32, pub end_line: u32, pub end_column: u32 }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FieldBinding { pub package: String, pub section: String, pub field: String, pub form: String, pub expected: Vec<String> }

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompilerResponse {
    ok: bool,
    model: Option<serde_json::Value>,
    targets: Vec<serde_json::Value>,
    witnesses: Vec<CompilerWitness>,
    diagnostics: Vec<serde_json::Value>,
}

pub struct CompilerRunner { executable: PathBuf }
impl CompilerRunner {
    pub fn new(installed_executable: PathBuf) -> Result<Self, AdmissionError> {
        Ok(Self { executable: installed_executable.canonicalize()? })
    }
    pub async fn compile(&self, source: &str) -> Result<CompiledDraft, AdmissionError> {
        if source.len() > MAX_SOURCE { return Err(AdmissionError::Limits); }
        let directory = tempfile::tempdir()?;
        std::fs::write(directory.path().join("intent.net"), source)?;
        let mut child = Command::new(&self.executable).arg("evaluate").arg("intent.net")
            .current_dir(directory.path()).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true).spawn()?;
        let stdout = child.stdout.take().ok_or(AdmissionError::Limits)?;
        let stderr = child.stderr.take().ok_or(AdmissionError::Limits)?;
        let work = async {
            let read_stdout = async { let mut b = Vec::new(); stdout.take((MAX_OUTPUT + 1) as u64).read_to_end(&mut b).await?; Ok::<_, std::io::Error>(b) };
            let read_stderr = async { let mut b = Vec::new(); stderr.take(65_537).read_to_end(&mut b).await?; Ok::<_, std::io::Error>(b) };
            let (out, err) = tokio::try_join!(read_stdout, read_stderr)?;
            let status = child.wait().await?;
            Ok::<_, std::io::Error>((out, err, status))
        };
        let (out, err, status) = timeout(Duration::from_secs(30), work).await.map_err(|_| AdmissionError::Limits)??;
        if out.len() > MAX_OUTPUT || err.len() > 65_536 { return Err(AdmissionError::Limits); }
        if !status.success() { return Err(AdmissionError::InvalidSource("compiler process failed".into())); }
        let reply: CompilerResponse = serde_json::from_slice(&out)?;
        if !reply.ok || reply.model.is_none() || !reply.diagnostics.is_empty() {
            return Err(AdmissionError::InvalidSource(serde_json::to_string(&reply.diagnostics)?));
        }
        let target_names: HashSet<_> = reply.targets.iter().filter_map(|t| t.get("target").and_then(|v| v.as_str())).collect();
        if target_names.len() != reply.targets.len() || reply.witnesses.len() != reply.targets.len()
            || reply.witnesses.iter().any(|w| w.version != 1 || !target_names.contains(w.target.as_str()))
            || reply.witnesses.iter().map(|w| &w.target).collect::<HashSet<_>>().len() != target_names.len()
        { return Err(AdmissionError::Binding); }
        Ok(CompiledDraft { source: source.to_owned(), source_digest: payload_digest(source.as_bytes()), model: reply.model.unwrap(), targets: reply.targets, witnesses: reply.witnesses })
    }
}

/// Only CompilerRunner constructs this record; the serialized browser preview
/// is not accepted as this token.
pub struct CompiledDraft {
    source: String,
    source_digest: String,
    model: serde_json::Value,
    targets: Vec<serde_json::Value>,
    witnesses: Vec<CompilerWitness>,
}
impl CompiledDraft {
    pub fn source_digest(&self) -> &str { &self.source_digest }
    pub fn model(&self) -> &serde_json::Value { &self.model }
    pub fn targets(&self) -> &[serde_json::Value] { &self.targets }
    pub fn witnesses(&self) -> &[CompilerWitness] { &self.witnesses }
    pub fn blockers(&self) -> Vec<String> {
        self.witnesses.iter().flat_map(|w| w.claims.iter()).filter_map(|c| c.unsupported.as_ref().map(|reason| format!("{}: {reason}", c.id))).collect()
    }
}

/// Explicit source bindings from the controller's authorized witness inventory.
/// Semantic expectations remain the compiler's responsibility.
pub struct PlanningBindings {
    pub sources: HashMap<String, intent_protocol::assurance::ProbeSource>,
}

/// Full admission remains fail-closed for unsupported compiler claims. Concrete
/// config requirements are generated here; operational requirements cannot be
/// substituted with readback by a submitted plan.
pub fn generate_claims(draft: &CompiledDraft, bindings: &PlanningBindings) -> Result<Vec<Claim>, AdmissionError> {
    let blockers = draft.blockers();
    if !blockers.is_empty() { return Err(AdmissionError::Coverage(blockers.join("; "))); }
    let mut claims = Vec::new();
    for witness in &draft.witnesses {
        for claim in &witness.claims {
            if !matches!(claim.kind.as_str(), "interface" | "addressing" | "attachment" | "route" | "bridge" | "dns-configuration") {
                return Err(AdmissionError::Coverage(format!("{}: unknown semantic claim kind {}", claim.id, claim.kind)));
            }
            let source = bindings.sources.get(&claim.target).ok_or_else(|| AdmissionError::Coverage(format!("{} needs an authorized witness source", claim.id)))?;
            let mut requirements = Vec::new();
            for binding in &claim.bindings {
                // Section-kind and absence assertions need typed readback predicates
                // before admission; never silently drop them from coverage.
                if binding.form == "section-kind" || binding.expected.is_empty() {
                    return Err(AdmissionError::Coverage(format!("{} requires section-kind/absence readback coverage", claim.id)));
                }
                requirements.push(ProbeRequirement { primitive: Primitive::UciReadback, source: source.clone(), endpoint: None,
                    expectation: Expectation::UciValue { package: binding.package.clone(), section: binding.section.clone(), option: binding.field.clone(), form: match binding.form.as_str() {
                        "scalar" => UciFieldForm::Scalar,
                        "ordered-list" => UciFieldForm::OrderedList,
                        _ => return Err(AdmissionError::Coverage("unknown compiler field form".into())),
                    }, values: binding.expected.clone() } });
            }
            // All currently derived classes also require operational corroboration.
            // The operational binding vocabulary is not complete yet.
            if matches!(claim.kind.as_str(), "interface" | "addressing" | "attachment" | "route" | "bridge" | "dns-configuration") {
                return Err(AdmissionError::Coverage(format!("{} requires complete operational corroboration", claim.id)));
            }
            claims.push(Claim { id: claim.id.clone(), device_id: claim.target.clone(), kind: claim.kind.clone(),
                source: SourceSpan { file: claim.source.file.clone(), line: claim.source.line, column: claim.source.column, end_line: claim.source.end_line, end_column: claim.source.end_column },
                requirements, unsupported_reason: None });
        }
    }
    Ok(claims)
}

pub struct AdmittedRevision { revision: RevisionRef, source: String, plan: AssurancePlan }
impl AdmittedRevision {
    pub fn revision(&self) -> &RevisionRef { &self.revision }
    pub fn source(&self) -> &str { &self.source }
    pub fn plan(&self) -> &AssurancePlan { &self.plan }
}

pub fn admit(draft: CompiledDraft, revision_id: String, plan: AssurancePlan, profiles: &[DeviceProfile], bindings: &PlanningBindings,
    expected_epoch: u64, expected_graph_version: u64, now_ms: u64) -> Result<AdmittedRevision, AdmissionError>
{
    if revision_id.is_empty() { return Err(AdmissionError::Binding); }
    let revision = RevisionRef { id: revision_id, source_digest: draft.source_digest.clone() };
    let claims = generate_claims(&draft, bindings)?;
    let context = CoverageContext { revision: &revision, epoch: expected_epoch, graph_version: expected_graph_version, profiles, claims: &claims, now_ms };
    check_plan(&plan, &context).map_err(|e| AdmissionError::Coverage(e.to_string()))?;
    Ok(AdmittedRevision { revision, source: draft.source, plan })
}
