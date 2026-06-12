use lopdf::content::{Content, Operation};
use lopdf::{dictionary, Document, Object, Stream};

pub fn minimal_valid_pdf() -> Vec<u8> {
    create_pdf_with_pages(1)
}

pub fn create_pdf_with_pages(page_count: usize) -> Vec<u8> {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });

    let mut page_ids = Vec::with_capacity(page_count);
    for index in 0..page_count {
        let content = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![50.into(), 750.into()]),
                Operation::new(
                    "Tj",
                    vec![Object::string_literal(format!("page-{index}"))],
                ),
                Operation::new("ET", vec![]),
            ],
        };
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
            "Resources" => resources_id,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
        });
        page_ids.push(Object::Reference(page_id));
    }

    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => page_ids,
        "Count" => page_count as i64,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));

    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    let mut buffer = Vec::new();
    doc.save_to(&mut buffer).expect("failed to serialize pdf");
    buffer
}

pub fn create_sized_pdf(target_bytes: usize) -> Vec<u8> {
    let mut pages = 1usize;

    loop {
        let bytes = create_pdf_with_pages(pages);
        if bytes.len() >= target_bytes || pages >= 2_000 {
            return bytes;
        }
        pages = pages.saturating_mul(2).max(pages + 25);
    }
}

pub fn not_a_pdf_bytes() -> Vec<u8> {
    b"This is plain text, not a PDF".to_vec()
}

pub fn corrupt_pdf_bytes() -> Vec<u8> {
    b"%PDF-1.4\nthis structure is intentionally broken".to_vec()
}

pub fn multipart_pdf_body(filename: &str, pdf_bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "----task-tools-test-boundary";
    let mut body = Vec::new();

    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/pdf\r\n\r\n");
    body.extend_from_slice(pdf_bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    (
        format!("multipart/form-data; boundary={boundary}"),
        body,
    )
}
