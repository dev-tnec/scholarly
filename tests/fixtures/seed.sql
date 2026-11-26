-- Seed data for the ignored route integration tests.
-- Run against a Neon branch after `scholarly` migrations, then point
-- DATABASE_URL at it and run `cargo test -- --ignored`.
-- All values come from the content-master-template worked example and are
-- example data, never for publication.

BEGIN;

DELETE FROM scholarships WHERE slug IN ('nl-scholarship', 'chevening', 'mext', 'gks');

INSERT INTO scholarships (slug, name, official_name, abbreviation, provider, provider_type,
    location, country, levels, funding_type, open_date, close_date, deadline_type,
    official_url, checked_at, quota, duration, language_requirement, visa_requirement,
    research_proposal, title_tag, meta_description, notes, popularity, published)
VALUES
('nl-scholarship', 'NL Scholarship', 'NL Scholarship', 'NLS', 'Nuffic Neso Indonesia', 'Yayasan',
    'Luar Negeri', 'Belanda', ARRAY['S1','S2'], 'Fully Funded', '2026-11-01', '2027-01-31', 'fixed',
    'https://www.nesoindonesia.or.id', CURRENT_DATE, '', 'Tahun pertama studi',
    'IELTS 6.5 atau TOEFL iBT 90', 'Ya', 'Ya untuk S2',
    'NL Scholarship 2027: Syarat, Cara Daftar, dan Deadline',
    'NL Scholarship 2027: biaya kuliah penuh, uang saku, tiket pesawat, dan asuransi untuk studi S1 dan S2 di Belanda. Cek syarat, dokumen, dan deadline di sini.',
    'NL Scholarship adalah beasiswa penuh dari Nuffic Neso Indonesia untuk mahasiswa Indonesia yang ingin menempuh studi S1 atau S2 di Belanda. Beasiswa ini menanggung biaya kuliah penuh dan memberi bantuan biaya hidup selama masa studi.

Kamu bebas memilih program studi dari daftar kampus Belanda peserta program, lalu mendaftar langsung ke kampus tujuan sebelum tenggat yang ditetapkan.',
    0, true),
('chevening', 'Chevening Scholarship', 'Chevening', '', 'Pemerintah Inggris (FCDO)', 'Pemerintah',
    'Luar Negeri', 'Inggris', ARRAY['S2'], 'Fully Funded', '2026-08-01', '2026-11-05', 'fixed',
    'https://www.chevening.org', CURRENT_DATE, '', 'Satu tahun', 'IELTS 6.5', 'Ya', '',
    'Chevening 2027: Syarat, Cara Daftar, dan Deadline',
    'Chevening 2027: biaya kuliah penuh, uang saku, tiket pesawat, dan asuransi untuk studi S2 di Inggris. Cek syarat, dokumen, dan deadline di sini.',
    'Beasiswa pemerintah Inggris untuk calon pemimpin Indonesia yang ingin menempuh studi S2.',
    0, true),
('mext', 'MEXT Japanese Government Scholarship', 'MEXT', 'MEXT', 'Kedutaan Besar Jepang', 'Pemerintah',
    'Luar Negeri', 'Jepang', ARRAY['S1','S2','S3'], 'Fully Funded', '2026-04-01', '2027-05-15', 'fixed',
    'https://www.studyinjapan.go.jp', CURRENT_DATE, '', 'Sesuai durasi program',
    'TOEFL atau JLPT sesuai kampus', 'Ya', 'Ya untuk S2 dan S3',
    'MEXT 2027: Syarat, Cara Daftar, dan Deadline',
    'MEXT 2027: biaya kuliah penuh, uang saku bulanan, dan tiket pesawat untuk studi S1 sampai S3 di Jepang. Cek syarat, dokumen, dan deadline di sini.',
    'Beasiswa pemerintah Jepang untuk studi S1 sampai S3 di Jepang.',
    0, true),
('gks', 'Global Korea Scholarship', 'Global Korea Scholarship (GKS)', 'GKS', 'NIIED Korea Selatan', 'Pemerintah',
    'Luar Negeri', 'Korea Selatan', ARRAY['S1','S2'], 'Fully Funded', '2026-09-01', '2027-03-20', 'fixed',
    'https://www.studyinkorea.go.kr', CURRENT_DATE, '', 'Sesuai durasi program',
    'TOPIK atau IELTS sesuai kampus', 'Ya', '',
    'GKS 2027: Syarat, Cara Daftar, dan Deadline',
    'GKS 2027: biaya kuliah penuh, uang saku, tiket pesawat, dan kursus bahasa Korea untuk studi S1 dan S2. Cek syarat, dokumen, dan deadline di sini.',
    'Beasiswa pemerintah Korea Selatan untuk mahasiswa Indonesia.',
    0, true);

INSERT INTO benefits (scholarship_id, item, detail, source_url, sort_order)
SELECT s.id, v.item, v.detail, '', v.ord
FROM scholarships s
JOIN (VALUES
    ('Biaya kuliah', 'Ditanggung penuh selama masa studi', 0),
    ('Uang saku bulanan', 'Sebesar nominal resmi dari penyelenggara', 1),
    ('Tiket pesawat', 'Sekali keberangkatan dan kepulangan', 2),
    ('Asuransi kesehatan', 'Tercakup selama periode studi', 3),
    ('Allowance buku', 'Belum dikonfirmasi, periksa situs resmi', 4),
    ('Biaya visa', 'Belum dikonfirmasi, periksa situs resmi', 5)
) AS v(item, detail, ord) ON s.slug = 'nl-scholarship';

INSERT INTO requirements (scholarship_id, group_name, text, source_url, sort_order)
SELECT s.id, v.group_name, v.text, '', v.ord
FROM scholarships s
JOIN (VALUES
    ('Syarat Umum', 'Kewarganegaraan Indonesia', 0),
    ('Syarat Umum', 'Lulusan SMA atau S1 sesuai jenjang yang dituju', 1),
    ('Syarat Umum', 'Usia maksimum sesuai ketentuan penyelenggara', 2),
    ('Syarat Akademik', 'IPK minimum sesuai ketentuan kampus tujuan', 3),
    ('Syarat Akademik', 'Skor IELTS atau TOEFL sesuai syarat program studi', 4)
) AS v(group_name, text, ord) ON s.slug = 'nl-scholarship';

INSERT INTO documents (scholarship_id, name, required, source_url, sort_order)
SELECT s.id, v.name, 1, '', v.ord
FROM scholarships s
JOIN (VALUES
    ('Ijazah dan transkrip yang dilegalisir', 0),
    ('Paspor', 1),
    ('Skor IELTS atau TOEFL', 2),
    ('Surat rekomendasi', 3),
    ('Esai motivasi', 4),
    ('Proposal penelitian untuk S2 dan S3', 5)
) AS v(name, ord) ON s.slug = 'nl-scholarship';

INSERT INTO steps (scholarship_id, step_no, title, detail)
SELECT s.id, v.step_no, v.title, v.detail
FROM scholarships s
JOIN (VALUES
    (1, 'Cek Kelayakan', 'Pastikan kamu memenuhi syarat kewarganegaraan, jenjang, dan IPK minimum.'),
    (2, 'Siapkan Dokumen', 'Kumpulkan ijazah, transkrip, skor IELTS, dan surat rekomendasi.'),
    (3, 'Buat Akun di Portal Resmi', 'Registrasi di portal kampus Belanda tujuanmu.'),
    (4, 'Isi Formulir dan Unggah Dokumen', 'Lengkapi data pendaftaran dan unggah semua berkas.'),
    (5, 'Submit Sebelum Deadline', 'Kirim pendaftaran, lalu periksa ulang statusnya di portal resmi.')
) AS v(step_no, title, detail) ON s.slug = 'nl-scholarship';

INSERT INTO timeline_events (scholarship_id, stage, event_date, event_label, note, sort_order)
SELECT s.id, v.stage, v.event_date, v.event_label, v.note, v.ord
FROM scholarships s
JOIN (VALUES
    ('Pembukaan pendaftaran', DATE '2026-11-01', '1 Nov 2026', 'Melalui portal kampus peserta', 0),
    ('Tenggat submit', DATE '2027-01-31', '31 Jan 2027', 'Batas akhir pengunggahan dokumen', 1),
    ('Seleksi administrasi', NULL, 'Feb 2027', 'Verifikasi berkas oleh kampus', 2),
    ('Pengumuman', NULL, 'Apr 2027', 'Diumumkan melalui email resmi', 3)
) AS v(stage, event_date, event_label, note, ord) ON s.slug = 'nl-scholarship';

INSERT INTO faqs (scholarship_id, question, answer, sort_order)
SELECT s.id, v.question, v.answer, v.ord
FROM scholarships s
JOIN (VALUES
    ('Apakah NL Scholarship menanggung semua biaya kuliah?',
     'Ya, biaya kuliah ditanggung penuh selama masa studi sesuai ketentuan penyelenggara. Rincian lengkapnya selalu bisa kamu cek di situs resmi.', 0),
    ('Apakah beasiswa ini bisa diperpanjang?',
     'Masa tanggungan mengikuti durasi program studi yang kamu ambil. Kalau masa studi kamu diperpanjang, ketentuannya berbeda dan harus kamu konfirmasi ke kampus.', 1),
    ('Apakah bisa digabung dengan beasiswa lain?',
     'Sebagian besar kampus peserta tidak mengizinkan kombinasi dengan beasiswa lain untuk komponen yang sama. Konfirmasi ke penyalur resmi sebelum mengajukan.', 2),
    ('Berapa skor IELTS yang dibutuhkan?',
     'Syarat skor mengikuti program studi pilihanmu, dan setiap kampus menetapkan angkanya sendiri. Cek halaman program sebelum mendaftar.', 3)
) AS v(question, answer, ord) ON s.slug = 'nl-scholarship';

INSERT INTO related (scholarship_id, related_id)
SELECT a.id, b.id
FROM scholarships a, scholarships b
WHERE a.slug = 'nl-scholarship'
  AND b.slug IN ('chevening', 'mext', 'gks');

COMMIT;
