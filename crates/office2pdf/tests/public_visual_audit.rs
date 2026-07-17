#![cfg(not(target_arch = "wasm32"))]

mod common;

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use office2pdf::config::{ConvertOptions, Format};

#[derive(Debug, serde::Deserialize)]
struct VisualAuditManifest {
    format: String,
    cases: Vec<VisualAuditCase>,
}

#[derive(Debug, serde::Deserialize)]
struct VisualAuditCase {
    id: String,
    fixture: String,
    focus: String,
}

#[derive(serde::Serialize)]
struct VisualAuditReport<'a> {
    format: &'a str,
    dpi: u32,
    cases: Vec<VisualAuditResult>,
}

#[derive(serde::Serialize)]
struct VisualAuditResult {
    id: String,
    fixture: String,
    focus: String,
    status: String,
    ground_truth_pages: usize,
    output_pages: usize,
    ground_truth_text_length: usize,
    output_text_length: usize,
    ground_truth_images: Vec<String>,
    output_images: Vec<String>,
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_visual_audit_manifest(path: &Path) -> VisualAuditManifest {
    let data = std::fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("read visual audit manifest {}: {error}", path.display()));
    serde_json::from_str(&data)
        .unwrap_or_else(|error| panic!("parse visual audit manifest {}: {error}", path.display()))
}

fn render_pdf_to_jpegs(pdf_path: &Path, output_dir: &Path, prefix: &str, dpi: u32) -> Vec<PathBuf> {
    std::fs::create_dir_all(output_dir).expect("create visual audit output directory");
    let output_prefix = output_dir.join(prefix);
    let status = Command::new("pdftoppm")
        .args([
            "-jpeg",
            "-jpegopt",
            "quality=86,progressive=y",
            "-r",
            &dpi.to_string(),
        ])
        .arg(pdf_path)
        .arg(&output_prefix)
        .status()
        .expect("run pdftoppm");
    assert!(
        status.success(),
        "pdftoppm failed for {}",
        pdf_path.display()
    );

    let mut images: Vec<PathBuf> = std::fs::read_dir(output_dir)
        .expect("read visual audit output directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension().is_some_and(|extension| extension == "jpg")
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with(prefix))
        })
        .collect();
    images.sort();
    images
}

fn relative_image_paths(images: &[PathBuf], report_dir: &Path) -> Vec<String> {
    images
        .iter()
        .map(|path| {
            path.strip_prefix(report_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect()
}

fn generate_powerpoint_ground_truth(
    manifest: &VisualAuditManifest,
    fixtures_dir: &Path,
    ground_truth_dir: &Path,
) {
    assert_eq!(
        std::env::consts::OS,
        "macos",
        "Microsoft PowerPoint GT export is only available on macOS"
    );
    std::fs::create_dir_all(ground_truth_dir).expect("create PowerPoint GT directory");
    let script = project_root().join("scripts/macos/export_powerpoint_pdfs.applescript");
    let mut command = Command::new("osascript");
    command.arg(script).arg(ground_truth_dir);
    for case in &manifest.cases {
        command.arg(&case.id).arg(fixtures_dir.join(&case.fixture));
    }
    let output = command.output().expect("run PowerPoint GT exporter");
    assert!(
        output.status.success(),
        "PowerPoint GT export failed:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn pptx_visual_audit_manifest_covers_priority_areas() {
    let manifest_path = project_root().join("tests/visual_audits/pptx.json");
    let manifest = load_visual_audit_manifest(&manifest_path);

    assert_eq!(manifest.format, "pptx");
    assert!(manifest.cases.len() >= 8);
    let fixtures_dir = project_root().join("tests/fixtures");
    let mut ids: BTreeSet<&str> = BTreeSet::new();
    for case in &manifest.cases {
        assert!(
            ids.insert(&case.id),
            "duplicate PPTX visual audit id: {}",
            case.id
        );
        assert!(
            case.id
                .chars()
                .all(|character| character.is_ascii_lowercase()
                    || character.is_ascii_digit()
                    || character == '-'),
            "visual audit id must be lowercase ASCII kebab-case: {}",
            case.id
        );
        assert!(
            fixtures_dir.join(&case.fixture).is_file(),
            "missing PPTX visual audit fixture: {}",
            case.fixture
        );
    }
    for focus in [
        "group transforms",
        "image crop",
        "master and layout",
        "theme table",
        "image transparency",
        "SmartArt",
        "chart",
        "text rotation",
    ] {
        assert!(
            manifest.cases.iter().any(|case| case.focus == focus),
            "missing PPTX visual audit focus: {focus}"
        );
    }
}

#[test]
#[ignore]
fn test_public_pptx_visual_audit() {
    assert!(
        common::is_pdftoppm_available(),
        "pdftoppm (poppler-utils) is required"
    );
    assert!(
        common::is_pdftotext_available(),
        "pdftotext (poppler-utils) is required"
    );

    let dpi: u32 = std::env::var("VISUAL_DPI")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(150);
    let manifest =
        load_visual_audit_manifest(&project_root().join("tests/visual_audits/pptx.json"));
    let fixtures_dir = project_root().join("tests/fixtures");
    let report_dir = std::env::var_os("VISUAL_AUDIT_DIR").map_or_else(
        || project_root().join("target/visual-audit/pptx"),
        PathBuf::from,
    );
    let ground_truth_dir = report_dir.join("ground-truth-pdf");
    std::fs::create_dir_all(&report_dir).expect("create visual audit report directory");

    if std::env::var("GENERATE_MICROSOFT_GT").as_deref() == Ok("1") {
        generate_powerpoint_ground_truth(&manifest, &fixtures_dir, &ground_truth_dir);
    }

    let mut results: Vec<VisualAuditResult> = Vec::new();
    for case in &manifest.cases {
        let fixture_path = fixtures_dir.join(&case.fixture);
        let ground_truth_pdf = ground_truth_dir.join(format!("{}.pdf", case.id));
        assert!(
            ground_truth_pdf.is_file(),
            "missing Microsoft PowerPoint GT PDF for {}: run with GENERATE_MICROSOFT_GT=1",
            case.id
        );

        let case_dir = report_dir.join(&case.id);
        if case_dir.exists() {
            std::fs::remove_dir_all(&case_dir).expect("clean visual audit case directory");
        }
        std::fs::create_dir_all(&case_dir).expect("create visual audit case directory");

        let input = std::fs::read(&fixture_path).expect("read PPTX visual audit fixture");
        let conversion =
            office2pdf::convert_bytes(&input, Format::Pptx, &ConvertOptions::default());
        let Ok(conversion) = conversion else {
            results.push(VisualAuditResult {
                id: case.id.clone(),
                fixture: case.fixture.clone(),
                focus: case.focus.clone(),
                status: format!("conversion_error: {}", conversion.unwrap_err()),
                ground_truth_pages: 0,
                output_pages: 0,
                ground_truth_text_length: 0,
                output_text_length: 0,
                ground_truth_images: Vec::new(),
                output_images: Vec::new(),
            });
            continue;
        };

        let output_pdf = case_dir.join("office2pdf.pdf");
        std::fs::write(&output_pdf, conversion.pdf).expect("write office2pdf audit PDF");
        let ground_truth_images = render_pdf_to_jpegs(&ground_truth_pdf, &case_dir, "gt", dpi);
        let output_images = render_pdf_to_jpegs(&output_pdf, &case_dir, "output", dpi);
        let ground_truth_text = common::extract_text_from_pdf_file(&ground_truth_pdf);
        let output_text = common::extract_text_from_pdf_file(&output_pdf);

        results.push(VisualAuditResult {
            id: case.id.clone(),
            fixture: case.fixture.clone(),
            focus: case.focus.clone(),
            status: "ok".to_string(),
            ground_truth_pages: ground_truth_images.len(),
            output_pages: output_images.len(),
            ground_truth_text_length: ground_truth_text.len(),
            output_text_length: output_text.len(),
            ground_truth_images: relative_image_paths(&ground_truth_images, &report_dir),
            output_images: relative_image_paths(&output_images, &report_dir),
        });
    }

    let report = VisualAuditReport {
        format: &manifest.format,
        dpi,
        cases: results,
    };
    let report_json = serde_json::to_string_pretty(&report).expect("serialize visual audit report");
    std::fs::write(report_dir.join("report.json"), format!("{report_json}\n"))
        .expect("write visual audit report");
    println!("PPTX visual audit report: {}", report_dir.display());
}
