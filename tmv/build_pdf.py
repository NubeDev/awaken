#!/usr/bin/env python3
"""Build the TMV HW/FW spec PDF from markdown using WeasyPrint.

Usage: python3 build_pdf.py
Outputs: Nube_iO_TMVM_Gen2_HW_FW_Spec.pdf
"""
import base64
import pathlib
import markdown
from weasyprint import HTML

HERE = pathlib.Path(__file__).parent
MD = HERE / "tmv-hw-fw-spec.md"
LOGO = HERE / "assets" / "logo.svg"
OUT = HERE / "Nube_iO_TMVM_Gen2_HW_FW_Spec.pdf"

ACCENT = "#0098A6"
INK = "#1F2D3D"

logo_b64 = base64.b64encode(LOGO.read_bytes()).decode()
logo_uri = f"data:image/svg+xml;base64,{logo_b64}"

body_html = markdown.markdown(
    MD.read_text(),
    extensions=["tables", "fenced_code", "attr_list", "sane_lists"],
)

CSS = f"""
@page {{
  size: A4;
  margin: 15mm 15mm 16mm 15mm;
  @bottom-left  {{ content: "Nube iO — Commercial in Confidence";
                  font-size: 7.5pt; color: #8a97a6; font-family: 'Helvetica Neue', Arial, sans-serif; }}
  @bottom-center{{ content: "ACX · CliniMix-TMV HW/FW Spec";
                  font-size: 7.5pt; color: #8a97a6; font-family: 'Helvetica Neue', Arial, sans-serif; }}
  @bottom-right {{ content: "Page " counter(page) " of " counter(pages);
                  font-size: 7.5pt; color: #8a97a6; font-family: 'Helvetica Neue', Arial, sans-serif; }}
}}
@page :first {{
  @bottom-left {{ content: ""; }}
  @bottom-center {{ content: ""; }}
  @bottom-right {{ content: ""; }}
}}

html {{ font-family: 'Helvetica Neue', Arial, sans-serif; font-size: 9.4pt; color: {INK}; line-height: 1.34; }}
body {{ margin: 0; }}

h1 {{ font-size: 15pt; color: {INK}; border-bottom: 2.5px solid {ACCENT};
      padding-bottom: 3px; margin: 4px 0 9px 0; break-after: avoid; }}
h2 {{ font-size: 11.5pt; color: {ACCENT}; margin: 13px 0 5px 0; break-after: avoid; }}
h3 {{ font-size: 10pt; color: {INK}; margin: 10px 0 3px 0; break-after: avoid; }}
p  {{ margin: 0 0 7px 0; }}
strong {{ color: {INK}; }}
a {{ color: {ACCENT}; text-decoration: none; }}

ul {{ margin: 4px 0 8px 0; padding-left: 18px; }}
li {{ margin: 2px 0; }}

table {{ border-collapse: collapse; width: 100%; margin: 6px 0 10px 0;
         font-size: 8.2pt; break-inside: auto; }}
th {{ background: {ACCENT}; color: #fff; text-align: left; padding: 3.5px 6px;
      font-weight: 600; border: 1px solid {ACCENT}; }}
td {{ padding: 3.5px 6px; border: 1px solid #d6dde4; vertical-align: top; }}
tr {{ break-inside: avoid; }}
tbody tr:nth-child(even) {{ background: #f3f7f8; }}

pre {{ background: #f5f7f9; border: 1px solid #d6dde4; border-left: 3px solid {ACCENT};
       border-radius: 3px; padding: 10px 12px; font-size: 7.6pt; line-height: 1.25;
       overflow: visible; white-space: pre; break-inside: avoid;
       font-family: 'DejaVu Sans Mono', 'Courier New', monospace; }}
code {{ font-family: 'DejaVu Sans Mono', 'Courier New', monospace; font-size: 8.6pt; }}
p code, li code, td code {{ background: #eef2f4; padding: 0 3px; border-radius: 2px; }}

.page-break {{ break-before: page; }}

/* Cover page */
.cover {{ break-after: page; padding-top: 30mm; }}
.cover img {{ width: 230px; }}
.cover .title {{ font-size: 26pt; font-weight: 700; color: {INK}; margin: 28mm 0 4px 0; line-height: 1.15; }}
.cover .subtitle {{ font-size: 13pt; color: {ACCENT}; font-weight: 600; margin-bottom: 2px; }}
.cover .tagline {{ font-size: 10.5pt; color: #5a6b7b; margin-bottom: 24mm; }}
.cover table {{ width: 100%; font-size: 9.5pt; }}
.cover .ctrl th {{ width: 34%; background: {INK}; color: #fff; border-color: {INK}; }}
.cover .accentbar {{ height: 5px; background: {ACCENT}; width: 100%; margin: 6px 0 0 0; }}
"""

cover = f"""
<div class="cover">
  <img src="{logo_uri}" alt="Nube iO"/>
  <div class="accentbar"></div>
  <div class="title">TMV Monitoring Device<br/>Hardware &amp; Firmware<br/>Technical Specification</div>
  <div class="subtitle">Nube iO ACX platform — Galvin CliniMix-TMV (Generation 2)</div>
  <div class="tagline">Device-level reverse specification — Nube iO Hardware &amp; Firmware Team</div>
  <table class="ctrl">
    <tr><th>Document</th><td>Nube iO ACX (ACX-001) — CliniMix-TMV Hardware &amp; Firmware Specification</td></tr>
    <tr><th>Prepared by</th><td>Nube iO — Hardware &amp; Firmware Team</td></tr>
    <tr><th>Date</th><td>16 June 2026</td></tr>
    <tr><th>Status</th><td>Draft V0.2 — incorporates Product Overview (ACX-001) &amp; Feedback Loop</td></tr>
    <tr><th>Responds to</th><td>Galvin Engineering — Temperature Monitoring Gen 2 Design Scope,
        Doc 004.00.00.30 Rev 0 (NPD/PDDR 1442N)</td></tr>
    <tr><th>Classification</th><td>Commercial in Confidence</td></tr>
  </table>
</div>
"""

html = f"<!doctype html><html><head><meta charset='utf-8'><style>{CSS}</style></head><body>{cover}{body_html}</body></html>"

HTML(string=html, base_url=str(HERE)).write_pdf(str(OUT))
print(f"Wrote {OUT}")
