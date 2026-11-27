<?xml version="1.0" encoding="UTF-8"?>
<xsl:stylesheet version="1.0"
  xmlns:xsl="http://www.w3.org/1999/XSL/Transform"
  xmlns:sitemap="http://www.sitemaps.org/schemas/sitemap/0.9"
  exclude-result-prefixes="sitemap">
<xsl:output method="html" encoding="UTF-8" indent="yes"/>
<xsl:template match="/">
<html lang="id">
<head>
<title>Peta Situs | Scholarly Titik Nol</title>
<meta name="robots" content="noindex"/>
<link rel="stylesheet" href="/static/css/site.v2.css"/>
<style>
  body{font-family:inherit;background:var(--surface-50,#f6f7f9);margin:0;padding:48px 16px}
  .sx-wrap{max-width:880px;margin:0 auto;background:#fff;border-radius:16px;border:1px solid var(--gray-200,#e2e5ea);overflow:hidden}
  .sx-head{background:var(--navy-900,#0e1b2c);color:#fff;padding:28px 32px}
  .sx-head h1{margin:0 0 4px;font-size:1.4rem}
  .sx-head p{margin:0;opacity:.7;font-size:.9rem}
  table{width:100%;border-collapse:collapse;font-size:.9rem}
  th{text-align:left;padding:12px 16px;background:var(--surface-50,#f6f7f9);color:var(--navy-900,#0e1b2c);font-weight:600;border-bottom:1px solid var(--gray-200,#e2e5ea)}
  td{padding:12px 16px;border-bottom:1px solid var(--gray-100,#eef0f3)}
  tr:last-child td{border-bottom:none}
  a{color:var(--navy-700,#1d3557);text-decoration:none}
  a:hover{text-decoration:underline}
  .sx-pri{font-weight:700;color:var(--red-600,#c1121f)}
</style>
</head>
<body>
<div class="sx-wrap">
  <div class="sx-head">
    <h1>Peta Situs Scholarly</h1>
    <p><xsl:value-of select="count(/sitemap:urlset/sitemap:url)"/> URL siap dijelajahi mesin pencari</p>
  </div>
  <table>
    <tr><th>URL</th><th>Terakhir Diperbarui</th><th>Frekuensi</th><th>Prioritas</th></tr>
    <xsl:for-each select="/sitemap:urlset/sitemap:url">
    <tr>
      <td><a href="{sitemap:loc}"><xsl:value-of select="sitemap:loc"/></a></td>
      <td><xsl:value-of select="sitemap:lastmod"/></td>
      <td><xsl:value-of select="sitemap:changefreq"/></td>
      <td class="sx-pri"><xsl:value-of select="sitemap:priority"/></td>
    </tr>
    </xsl:for-each>
  </table>
</div>
</body>
</html>
</xsl:template>
</xsl:stylesheet>
