use once_cell::sync::OnceCell;

use eyre::ContextCompat;
use fluvio_jolt::TransformSpec;
use fluvio_smartmodule::dataplane::smartmodule::SmartModuleInitError;
use fluvio_smartmodule::{
    dataplane::smartmodule::SmartModuleExtraParams, smartmodule, SmartModuleRecord, RecordData,
    Result,
};

static SPEC: OnceCell<TransformSpec> = OnceCell::new();

const PARAM_NAME: &str = "spec";

#[smartmodule(init)]
fn init(params: SmartModuleExtraParams) -> Result<()> {
    if let Some(raw_spec) = params.get(PARAM_NAME) {
        match serde_json::from_str(raw_spec) {
            Ok(spec) => {
                SPEC.set(spec).expect("spec is already initialized");
                Ok(())
            }
            Err(err) => {
                eprintln!("unable to parse spec from params: {err:?}");
                Err(eyre::Report::msg(
                    "could not parse the specification from `spec` param",
                ))
            }
        }
    } else {
        Err(SmartModuleInitError::MissingParam(PARAM_NAME.to_string()).into())
    }
}

#[smartmodule(map)]
pub fn map(record: &SmartModuleRecord) -> Result<(Option<RecordData>, RecordData)> {
    let spec = SPEC.get().wrap_err("jolt spec is not initialized")?;

    let key = record.key.clone();
    let record = serde_json::from_slice(record.value.as_ref())?;
    let transformed = fluvio_jolt::transform(record, spec)?;

    Ok((key, serde_json::to_vec(&transformed)?.into()))
}
