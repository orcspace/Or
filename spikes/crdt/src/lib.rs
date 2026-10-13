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

struct SeededRandom(u64);

impl SeededRandom {
    fn index(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % bound as u64) as usize
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let other = self.index(index + 1);
            values.swap(index, other);
        }
    }
}

/// Reproducible property workload with reordered, duplicated incremental packets.
/// Scalar text positions are intentionally not a production grapheme API.
pub fn randomized_convergence(
    seed_number: u64,
    edits_per_peer: usize,
) -> Result<usize, Box<dyn std::error::Error>> {
    let initial = new_replica(1)?;
    initial
        .get_text("blk_shared")
        .insert(0, "Base 😀 e\u{301} 中")?;
    for block in 0..12 {
        initial
            .get_movable_list("blocks")
            .push(format!("blk_{block:02}").as_str())?;
    }
    initial.commit();
    let snapshot = initial.export(ExportMode::Snapshot)?;
    let mut random = SeededRandom(seed_number | 1);
    let mut replicas = Vec::new();
    let mut packets = Vec::new();
    for peer in 2..6 {
        let replica = new_replica(peer)?;
        replica.import(&snapshot)?;
        for edit in 0..edits_per_peer {
            let before = replica.oplog_vv();
            let text = replica.get_text("blk_shared");
            let length = text.len_unicode();
            match random.index(6) {
                0 | 1 => {
                    let fragments = ["a", "Ж", "😀", "e\u{301}", "中", "ع"];
                    text.insert(random.index(length + 1), fragments[random.index(6)])?;
                }
                2 if length > 0 => text.delete(random.index(length), 1)?,
                3 if length > 0 => {
                    let start = random.index(length);
                    let end = start + 1 + random.index(length - start);
                    text.mark(start..end, "bold", random.index(2) == 0)?;
                }
                4 => {
                    replica
                        .get_movable_list("blocks")
                        .mov(random.index(12), random.index(12))?;
                }
                _ => {
                    replica.get_map("cells").insert(
                        &format!("row_{}:col_{}", random.index(8), random.index(4)),
                        format!("peer_{peer}:edit_{edit}").as_str(),
                    )?;
                }
            }
            replica.commit();
            packets.push(replica.export(ExportMode::updates(&before))?);
        }
        replicas.push(replica);
    }
    let mut pending_deliveries = 0;
    for replica in &replicas {
        random.shuffle(&mut packets);
        for packet in &packets {
            if replica.import(packet)?.pending.is_some() {
                pending_deliveries += 1;
            }
            replica.import(packet)?;
        }
    }
    // Confirm dependencies are drained instead of equating acceptance with application.
    let complete = replicas[0].export(ExportMode::all_updates())?;
    for replica in &replicas {
        // Check before a full resend: it must not silently repair lost packets.
        let before_resend = replica.oplog_vv();
        if before_resend != replicas[0].oplog_vv()
            || replica.get_deep_value() != replicas[0].get_deep_value()
            || replica.get_text("blk_shared").to_delta()
                != replicas[0].get_text("blk_shared").to_delta()
            || replica.import(&complete)?.pending.is_some()
            || replica.oplog_vv() != before_resend
        {
            return Err(std::io::Error::other(format!(
                "Randomized convergence failed for seed {seed_number}"
            ))
            .into());
        }
    }
    Ok(pending_deliveries)
}

#[derive(Debug)]
pub struct CompactionProbe {
    pub obsolete_update_rejected: bool,
    pub rejected_import_partially_applied: bool,
    pub archive_preserved_offline_edit: bool,
    pub shallow_preserved_formatting: bool,
    pub live_undo_steps: usize,
    pub full_snapshot_retained_undo_steps: usize,
    pub shallow_retained_undo_steps: usize,
}

/// Demonstrates why destructive history GC and import atomicity need policies.
pub fn compaction_probe() -> Result<CompactionProbe, Box<dyn std::error::Error>> {
    let archive = new_replica(1)?;
    let text = archive.get_text("blk_shared");
    text.insert(0, "base")?;
    text.mark(0..4, "bold", true)?;
    archive.commit();
    let offline = new_replica(2)?;
    offline.import(&archive.export(ExportMode::Snapshot)?)?;
    let offline_baseline = offline.oplog_vv();
    offline.get_text("blk_shared").insert(0, "OFFLINE ")?;
    offline.commit();
    let mut undo = UndoManager::new(&archive);
    undo.set_merge_interval(0);
    text.insert(0, "ONLINE ")?;
    archive.commit();
    let shallow = new_replica(3)?;
    shallow.import(&archive.export(ExportMode::shallow_snapshot(&archive.oplog_frontiers()))?)?;
    let shallow_preserved_formatting = shallow.get_text("blk_shared").to_delta() == text.to_delta();
    let shallow_undo = UndoManager::new(&shallow);
    let full_reload = new_replica(4)?;
    full_reload.import(&archive.export(ExportMode::Snapshot)?)?;
    let full_reload_undo = UndoManager::new(&full_reload);
    archive
        .get_map("metadata")
        .insert("fresh_after_cutoff", true)?;
    archive.commit();
    offline.import(&archive.export(ExportMode::updates(&offline_baseline))?)?;
    let mixed = offline.export(ExportMode::updates(&shallow.oplog_vv()))?;
    let before_failed_import = shallow.get_deep_value();
    let result = shallow.import(&mixed);
    let obsolete_update_rejected = matches!(
        result,
        Err(loro::LoroError::ImportUpdatesThatDependsOnOutdatedVersion)
    );
    let rejected_import_partially_applied = shallow.get_deep_value() != before_failed_import;
    archive.import(&mixed)?;
    offline.import(&archive.export(ExportMode::all_updates())?)?;
    let merged = text.to_string();
    let archive_preserved_offline_edit = merged.contains("OFFLINE ")
        && merged.contains("ONLINE ")
        && archive.get_deep_value() == offline.get_deep_value()
        && text.to_delta() == offline.get_text("blk_shared").to_delta();
    Ok(CompactionProbe {
        obsolete_update_rejected,
        rejected_import_partially_applied,
        archive_preserved_offline_edit,
        shallow_preserved_formatting,
        live_undo_steps: undo.undo_count(),
        full_snapshot_retained_undo_steps: full_reload_undo.undo_count(),
        shallow_retained_undo_steps: shallow_undo.undo_count(),
    })
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

    #[test]
    fn randomized_packets_converge_after_missing_dependencies_arrive()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut pending = 0;
        for seed in 1..=32 {
            pending += randomized_convergence(seed * 0x9e37_79b9, 64)?;
        }
        assert!(
            pending > 0,
            "Workload must exercise deferred dependency delivery"
        );
        Ok(())
    }

    #[test]
    fn history_gc_rejects_old_offline_edits_but_full_archive_preserves_them()
    -> Result<(), Box<dyn std::error::Error>> {
        let probe = compaction_probe()?;
        assert!(probe.obsolete_update_rejected);
        assert!(probe.archive_preserved_offline_edit);
        assert!(probe.shallow_preserved_formatting);
        assert!(probe.live_undo_steps > 0);
        assert_eq!(probe.full_snapshot_retained_undo_steps, 0);
        assert_eq!(probe.shallow_retained_undo_steps, 0);
        Ok(())
    }

    #[test]
    fn failed_mixed_shallow_import_is_not_an_atomic_transaction()
    -> Result<(), Box<dyn std::error::Error>> {
        let probe = compaction_probe()?;
        assert!(probe.obsolete_update_rejected);
        assert!(probe.rejected_import_partially_applied);
        Ok(())
    }
}
