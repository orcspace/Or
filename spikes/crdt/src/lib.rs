//! Experimental workloads and invariants, not the production document model.
//! Text positions here use Loro Unicode scalars; no grapheme-safe MCP claim.

use loro::{ExportMode, LoroDoc, LoroResult, StyleConfigMap, UndoManager};

pub fn new_replica(peer: u64) -> LoroResult<LoroDoc> {
    let doc = LoroDoc::new();
    doc.set_peer_id(peer)?;
    doc.config_text_style(StyleConfigMap::default_rich_text_config());
    Ok(doc)
}

/// One hundred explicit page groups with two formatted blocks per group.
/// Actual pagination is outside this model-only experiment.
pub fn docs_workload(pages: usize) -> LoroResult<LoroDoc> {
    let doc = new_replica(1)?;
    let order = doc.get_movable_list("blocks");
    let metadata = doc.get_map("page_breaks");
    let paragraph = "Русский текст. English paragraph. العربية. 日本語. 😀 e\u{301}. ".repeat(24);
    for page in 0..pages {
        metadata.insert(&format!("page_{page:04}"), true)?;
        for part in 0..2 {
            let id = format!("blk_{page:04}_{part}");
            order.push(id.as_str())?;
            let text = doc.get_text(id.as_str());
            text.insert(0, &paragraph)?;
            text.mark(0..7, "bold", true)?;
            text.mark(8..16, "italic", true)?;
        }
    }
    doc.commit();
    Ok(doc)
}

pub fn sheet_workload(rows: usize, columns: usize) -> LoroResult<LoroDoc> {
    let doc = new_replica(1)?;
    let row_order = doc.get_movable_list("rows");
    let column_order = doc.get_movable_list("columns");
    let cells = doc.get_map("cells");
    for row in 0..rows {
        row_order.push(format!("row_{row:06}").as_str())?;
    }
    for column in 0..columns {
        column_order.push(format!("col_{column:04}").as_str())?;
    }
    for row in 0..rows {
        for column in 0..columns {
            let key = format!("row_{row:06}:col_{column:04}");
            cells.insert(&key, (row * columns + column).to_string().as_str())?;
        }
    }
    doc.commit();
    Ok(doc)
}

pub fn slides_workload(count: usize) -> LoroResult<LoroDoc> {
    let doc = new_replica(1)?;
    let order = doc.get_movable_list("slides");
    let shapes = doc.get_map("shapes");
    for slide in 0..count {
        let id = format!("sld_{slide:04}");
        order.push(id.as_str())?;
        let notes = doc.get_text(format!("notes_{id}").as_str());
        notes.insert(0, "Заметки докладчика / Speaker notes")?;
        for shape in 0..5 {
            let shape_id = format!("shp_{slide:04}_{shape}");
            shapes.insert(&shape_id, id.as_str())?;
            doc.get_text(shape_id.as_str()).insert(0, "Slide content")?;
        }
    }
    doc.commit();
    Ok(doc)
}

pub fn concurrent_convergence() -> Result<(), Box<dyn std::error::Error>> {
    let seed = new_replica(1)?;
    seed.get_text("blk_shared").insert(0, "Совместный текст")?;
    seed.commit();
    let initial = seed.export(ExportMode::Snapshot)?;
    let baseline = seed.oplog_vv();
    let mut replicas = Vec::new();
    for peer in 2..5 {
        let doc = new_replica(peer)?;
        doc.import(&initial)?;
        for edit in 0..100 {
            doc.get_text("blk_shared").insert(0, "x")?;
            doc.get_map("cells")
                .insert(&format!("row_{edit}:col_{peer}"), true)?;
            doc.commit();
        }
        doc.get_text("blk_shared").mark(0..10, "bold", true)?;
        doc.commit();
        replicas.push(doc);
    }
    let updates = replicas
        .iter()
        .map(|doc| doc.export(ExportMode::updates(&baseline)))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, doc) in replicas.iter().enumerate() {
        // Different delivery order, with duplicate delivery to test idempotence.
        for offset in 0..updates.len() {
            let update = &updates[(index + offset) % updates.len()];
            doc.import(update)?;
            doc.import(update)?;
        }
    }
    for doc in &replicas[1..] {
        if doc.get_deep_value() != replicas[0].get_deep_value()
            || doc.get_text("blk_shared").to_delta()
                != replicas[0].get_text("blk_shared").to_delta()
        {
            return Err(std::io::Error::other("Replicas did not converge").into());
        }
    }
    Ok(())
}

pub fn local_undo_preserves_remote() -> Result<(), Box<dyn std::error::Error>> {
    let local = new_replica(1)?;
    local.get_text("blk_shared").insert(0, "base")?;
    local.commit();
    let remote = new_replica(2)?;
    remote.import(&local.export(ExportMode::Snapshot)?)?;
    let mut undo = UndoManager::new(&local);
    undo.set_merge_interval(0);
    local.get_text("blk_shared").insert(0, "LOCAL ")?;
    local.commit();
    remote.get_text("blk_shared").insert(0, "REMOTE ")?;
    remote.commit();
    local.import(&remote.export(ExportMode::all_updates())?)?;
    if !undo.undo()? || local.get_text("blk_shared").to_string() != "REMOTE base" {
        return Err(std::io::Error::other("Undo changed another participant's edit").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_replicas_preserve_text_marks_and_cells() -> Result<(), Box<dyn std::error::Error>>
    {
        concurrent_convergence()
    }

    #[test]
    fn undo_is_local_to_the_bound_peer() -> Result<(), Box<dyn std::error::Error>> {
        local_undo_preserves_remote()
    }

    #[test]
    fn unicode_units_are_explicit() -> LoroResult<()> {
        let doc = new_replica(1)?;
        let text = doc.get_text("blk_unicode");
        text.insert(0, "A😀e\u{301}")?;
        assert_eq!(text.len_unicode(), 4);
        assert_eq!(text.len_utf16(), 5);
        assert_eq!(text.len_utf8(), 8);
        text.insert(1, "中")?;
        assert_eq!(text.to_string(), "A中😀e\u{301}");
        Ok(())
    }

    #[test]
    fn snapshot_restores_content_and_formatting() -> Result<(), Box<dyn std::error::Error>> {
        let source = docs_workload(2)?;
        let restored = new_replica(2)?;
        restored.import(&source.export(ExportMode::Snapshot)?)?;
        assert_eq!(source.get_deep_value(), restored.get_deep_value());
        assert_eq!(
            source.get_text("blk_0000_0").to_delta(),
            restored.get_text("blk_0000_0").to_delta()
        );
        Ok(())
    }
}
