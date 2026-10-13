use loro::ExportMode;
use std::time::Instant;
use trinedocs_crdt_spike::{
    concurrent_convergence, docs_workload, local_undo_preserves_remote, new_replica,
    sheet_workload, slides_workload,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let case = std::env::args().nth(1).unwrap_or_else(|| "docs".to_owned());
    let started = Instant::now();
    let doc = match case.as_str() {
        "docs" => docs_workload(100)?,
        "sheets" => sheet_workload(1_000, 100)?,
        "slides" => slides_workload(100)?,
        "history" => {
            let doc = new_replica(1)?;
            for edit in 0..1_000_000 {
                doc.get_map("history_probe").insert("counter", edit)?;
                doc.commit();
            }
            doc
        }
        "merge" => {
            concurrent_convergence()?;
            local_undo_preserves_remote()?;
            println!(
                "{{\"case\":\"merge\",\"converged\":true,\"local_undo\":true,\"elapsed_ms\":{}}}",
                started.elapsed().as_secs_f64() * 1_000.0
            );
            return Ok(());
        }
        _ => {
            return Err(
                std::io::Error::other("Cases: docs, sheets, slides, history, merge").into(),
            );
        }
    };
    let create_ms = started.elapsed().as_secs_f64() * 1_000.0;
    let save_started = Instant::now();
    let snapshot = doc.export(ExportMode::Snapshot)?;
    let snapshot_ms = save_started.elapsed().as_secs_f64() * 1_000.0;
    let restored = new_replica(2)?;
    let open_started = Instant::now();
    restored.import(&snapshot)?;
    let import_ms = open_started.elapsed().as_secs_f64() * 1_000.0;
    let restored_value = restored.get_deep_value();
    let mut restored_styles = Vec::new();
    // Snapshot import may be lazy. Include first content/style materialization.
    if case == "docs" {
        for page in 0..100 {
            for part in 0..2 {
                let id = format!("blk_{page:04}_{part}");
                let delta = restored.get_text(id.as_str()).to_delta();
                restored_styles.push((id, delta));
            }
        }
    }
    let open_ms = open_started.elapsed().as_secs_f64() * 1_000.0;
    if restored_value != doc.get_deep_value() {
        return Err(std::io::Error::other("Snapshot content mismatch").into());
    }
    for (id, delta) in restored_styles {
        if delta != doc.get_text(id.as_str()).to_delta() {
            return Err(std::io::Error::other("Snapshot formatting mismatch").into());
        }
    }
    let shallow_started = Instant::now();
    let shallow = doc.export(ExportMode::shallow_snapshot(&doc.oplog_frontiers()))?;
    let shallow_ms = shallow_started.elapsed().as_secs_f64() * 1_000.0;
    println!(
        "{{\"case\":\"{case}\",\"os\":\"{}\",\"architecture\":\"{}\",\"create_ms\":{create_ms},\"snapshot_ms\":{snapshot_ms},\"import_ms\":{import_ms},\"open_ms\":{open_ms},\"snapshot_bytes\":{},\"shallow_bytes\":{},\"shallow_ms\":{shallow_ms},\"operations\":{},\"changes\":{}}}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        snapshot.len(),
        shallow.len(),
        doc.len_ops(),
        doc.len_changes()
    );
    Ok(())
}
