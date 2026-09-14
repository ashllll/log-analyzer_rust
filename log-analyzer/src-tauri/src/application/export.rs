//! ExportUseCase — stream a completed search session to CSV or JSON.
//!
//! Results stay in the backend result store and are fetched page by page, so
//! exporting does not depend on which pages the frontend has loaded.

use std::io::{BufWriter, Write};
use std::path::Path;

use la_core::error::CommandError;
use la_core::models::LogEntry;
use la_search::SearchPageResult;

const EXPORT_PAGE_SIZE: usize = 10_000;

fn export_error(code: &str, message: impl Into<String>) -> CommandError {
    CommandError::new(code, message.into())
}

fn write_csv_field(writer: &mut impl Write, value: &str) -> std::io::Result<()> {
    writer.write_all(b"\"")?;
    writer.write_all(value.replace('"', "\"\"").as_bytes())?;
    writer.write_all(b"\"")
}

fn write_csv_entry(writer: &mut impl Write, entry: &LogEntry) -> std::io::Result<()> {
    write!(writer, "{},", entry.id)?;
    write_csv_field(writer, &entry.timestamp)?;
    writer.write_all(b",")?;
    write_csv_field(writer, &entry.level)?;
    writer.write_all(b",")?;
    write_csv_field(writer, &entry.file)?;
    write!(writer, ",{},", entry.line)?;
    write_csv_field(writer, &entry.content)?;
    writer.write_all(b"\n")
}

fn persist_export(
    destination: &Path,
    write_contents: impl FnOnce(&mut BufWriter<&mut std::fs::File>) -> Result<(), CommandError>,
) -> Result<(), CommandError> {
    let parent = destination
        .parent()
        .ok_or_else(|| export_error("EXPORT_PATH_UNSAFE", "导出路径缺少父目录"))?;

    let mut temporary = tempfile::Builder::new()
        .prefix(".log-export-")
        .tempfile_in(parent)
        .map_err(|e| export_error("IO_ERROR", e.to_string()))?;

    {
        let mut writer = BufWriter::new(temporary.as_file_mut());
        write_contents(&mut writer)?;
        writer
            .flush()
            .map_err(|e| export_error("IO_ERROR", e.to_string()))?;
    }
    temporary
        .as_file()
        .sync_all()
        .map_err(|e| export_error("IO_ERROR", e.to_string()))?;

    temporary
        .persist(destination)
        .map_err(|e| export_error("IO_ERROR", e.error.to_string()))?;
    Ok(())
}

/// Export all entries in a completed search session using a paged reader.
pub fn export_search_results(
    destination: &Path,
    format: &str,
    mut fetch_page: impl FnMut(usize, usize) -> Result<SearchPageResult, CommandError>,
) -> Result<usize, CommandError> {
    let first_page = fetch_page(0, EXPORT_PAGE_SIZE)?;
    if !first_page.is_complete {
        return Err(export_error(
            "SEARCH_NOT_COMPLETE",
            "搜索仍在进行中，请等待完成后再导出",
        ));
    }
    let total_count = first_page.total_count;

    persist_export(destination, |writer| {
        match format {
            "csv" => {
                writer
                    .write_all(b"\xEF\xBB\xBFID,Timestamp,Level,File,Line,Content\n")
                    .map_err(|e| export_error("IO_ERROR", e.to_string()))?;
            }
            "json" => {
                let metadata = serde_json::json!({
                    "exportTime": chrono::Utc::now().to_rfc3339(),
                    "totalCount": total_count,
                });
                write!(writer, "{{\"metadata\":{metadata},\"results\":[")
                    .map_err(|e| export_error("IO_ERROR", e.to_string()))?;
            }
            _ => {
                return Err(export_error(
                    "UNSUPPORTED_EXPORT_FORMAT",
                    format!("Unsupported format: {format}"),
                ));
            }
        }

        let mut page = first_page;
        let mut written = 0usize;
        loop {
            for entry in &page.entries {
                match format {
                    "csv" => write_csv_entry(writer, entry)
                        .map_err(|e| export_error("IO_ERROR", e.to_string()))?,
                    "json" => {
                        if written > 0 {
                            writer
                                .write_all(b",")
                                .map_err(|e| export_error("IO_ERROR", e.to_string()))?;
                        }
                        serde_json::to_writer(&mut *writer, entry)
                            .map_err(|e| export_error("SERIALIZATION_ERROR", e.to_string()))?;
                    }
                    _ => unreachable!("format validated before paging"),
                }
                written += 1;
            }

            if !page.has_more {
                break;
            }
            let next_offset = page
                .next_offset
                .ok_or_else(|| export_error("EXPORT_PAGE_ERROR", "搜索结果分页缺少下一页偏移量"))?;
            if next_offset <= written.saturating_sub(page.entries.len()) {
                return Err(export_error(
                    "EXPORT_PAGE_ERROR",
                    "搜索结果分页偏移量未向前推进",
                ));
            }
            page = fetch_page(next_offset, EXPORT_PAGE_SIZE)?;
            if !page.is_complete {
                return Err(export_error(
                    "SEARCH_NOT_COMPLETE",
                    "搜索结果状态在导出期间发生变化",
                ));
            }
        }

        if written != total_count {
            return Err(export_error(
                "EXPORT_COUNT_MISMATCH",
                format!("导出条数不一致：预期 {total_count}，实际 {written}"),
            ));
        }
        if format == "json" {
            writer
                .write_all(b"]}")
                .map_err(|e| export_error("IO_ERROR", e.to_string()))?;
        }
        Ok(())
    })?;

    Ok(total_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn entry(id: usize) -> LogEntry {
        LogEntry {
            id,
            timestamp: Arc::from("2026-01-01T00:00:00Z"),
            level: Arc::from("INFO"),
            file: Arc::from("a,\"b.log"),
            real_path: Arc::from("/tmp/test.log"),
            line: id + 1,
            content: Arc::from(format!("line {id}, \"quoted\"")),
            tags: vec![],
            match_details: None,
            matched_keywords: None,
        }
    }

    fn page(entries: Vec<LogEntry>, total: usize, next: Option<usize>) -> SearchPageResult {
        SearchPageResult {
            entries,
            total_count: total,
            is_complete: true,
            has_more: next.is_some(),
            next_offset: next,
        }
    }

    #[test]
    fn exports_every_backend_page_to_json() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("results.json");
        let count = export_search_results(&destination, "json", |offset, _| match offset {
            0 => Ok(page(vec![entry(0), entry(1)], 3, Some(2))),
            2 => Ok(page(vec![entry(2)], 3, None)),
            _ => panic!("unexpected offset {offset}"),
        })
        .unwrap();

        let value: serde_json::Value =
            serde_json::from_slice(&std::fs::read(destination).unwrap()).unwrap();
        assert_eq!(count, 3);
        assert_eq!(value["metadata"]["totalCount"], 3);
        assert_eq!(value["results"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn csv_has_one_bom_and_escapes_fields() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("results.csv");
        export_search_results(&destination, "csv", |_, _| {
            Ok(page(vec![entry(0)], 1, None))
        })
        .unwrap();

        let bytes = std::fs::read(destination).unwrap();
        assert!(bytes.starts_with(b"\xEF\xBB\xBFID,"));
        assert!(!bytes[3..].starts_with(b"\xEF\xBB\xBF"));
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"a,\"\"b.log\""));
        assert!(text.contains("\"line 0, \"\"quoted\"\"\""));
    }

    #[test]
    fn incomplete_search_is_not_exported() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("results.json");
        let result = export_search_results(&destination, "json", |_, _| {
            Ok(SearchPageResult {
                entries: vec![entry(0)],
                total_count: 1,
                is_complete: false,
                has_more: false,
                next_offset: None,
            })
        });

        assert!(result.is_err());
        assert!(!destination.exists());
    }

    #[test]
    fn failed_publish_preserves_existing_destination() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("results.json");
        std::fs::create_dir(&destination).unwrap();
        std::fs::write(destination.join("marker"), b"keep").unwrap();

        let result = export_search_results(&destination, "json", |_, _| {
            Ok(page(vec![entry(0)], 1, None))
        });

        assert!(result.is_err());
        assert_eq!(std::fs::read(destination.join("marker")).unwrap(), b"keep");
    }
}
