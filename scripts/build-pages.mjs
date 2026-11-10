// Renders the project page into docs/index.html (English) and docs/<lang>.html, and
// docs/sitemap.xml. Every page is plain HTML in its own language, with the app and its
// questions described for search engines and AI answers (schema.org JSON-LD), so readers
// without JavaScript, crawlers and assistants all see the same page.
import { openSync, readFileSync, readSync, closeSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ORDER, STRINGS } from '../docs/site/strings.mjs';
import { ENGINES, GROUPS, STRESS, voiceName } from '../docs/site/samples.mjs';
import { STUDIO } from '../docs/site/studio.mjs';
import { ASR_BENCH, ENGINE_BENCH, LIVE_BENCH } from '../docs/site/engines.mjs';
// the studio's version, from the one place it is written
const VERSION = JSON.parse(readFileSync(new URL('../desktop/src-tauri/tauri.conf.json', import.meta.url), 'utf8')).version;
// the release's packages by system, named as the release scripts name them; `ends` finds each in the latest release
const PACKAGES = [
  { id: 'windows', file: `Speech.Studio_${VERSION}_x64-setup.exe`, ends: '-setup.exe' },
  { id: 'macos', file: `Speech-Studio-${VERSION}-macos-arm64.dmg`, ends: '-macos-arm64.dmg' },
  { id: 'appimage', file: `Speech-Studio-${VERSION}-linux-x86_64-experimental.AppImage`, ends: '.AppImage' },
  { id: 'deb', file: `Speech-Studio-${VERSION}-linux-amd64-experimental.deb`, ends: '.deb' },
];

const docs = join(dirname(fileURLToPath(import.meta.url)), '..', 'docs');
const css = readFileSync(join(docs, 'site', 'page.css'), 'utf8');
const { name: NAME, repo: REPO, site: SITE } = STUDIO;
// The donation page is the same on every studio site; this site only frames it.
const DONATE = Object.fromEntries(['donate.css', 'donate-lang.html', 'donate-body.html']
  .map((name) => [name, readFileSync(join(docs, 'site', name), 'utf8').trimEnd()]));

// The author's voice studios as tabs over the page: the same language on each site.
const FAMILY = [
  ['speech', 'Speech Studio', 'https://timoncool.github.io/speech-studio/'],
  ['dub', 'Dub Studio', 'https://timoncool.github.io/dub-studio/'],
];

const escape = (text) => String(text)
  .replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const file = (lang) => (lang === 'en' ? 'index.html' : `${lang}.html`);
const pageUrl = (lang) => `${SITE}${lang === 'en' ? '' : file(lang)}`;
const shot = (lang, name) => `screenshots/${lang}-${name}.webp`;
/** A screenshot's width and height, from its WebP header (lossy VP8, lossless VP8L or extended VP8X). */
function pictureSize(lang, name) {
  const handle = openSync(join(docs, 'screenshots', `${lang}-${name}.webp`), 'r');
  const head = Buffer.alloc(30);
  readSync(handle, head, 0, 30, 0);
  closeSync(handle);
  const chunk = head.toString('ascii', 12, 16);
  if (chunk === 'VP8X') return [1 + head.readUIntLE(24, 3), 1 + head.readUIntLE(27, 3)];
  if (chunk === 'VP8L') {
    const bits = head.readUInt32LE(21);
    return [(bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1];
  }
  if (chunk === 'VP8 ') return [head.readUInt16LE(26) & 0x3fff, head.readUInt16LE(28) & 0x3fff];
  throw new Error(`[ERROR] ${lang}-${name}.webp is not a WebP picture`);
}
const starsBadge = (alt) => `<img class="stars" src="https://img.shields.io/github/stars/${REPO.replace('https://github.com/', '')}?style=flat&amp;label=%E2%98%85&amp;color=3f3f46" alt="${alt}" height="20" />`;

/** The first sentence of a text: the one line a tile or the hero has room for. */
function firstSentence(text) {
  const cjk = text.match(/^(.+?[。！？])/);
  const latin = text.match(/^(.+?[.!?])(\s|$)/);
  const cut = [cjk?.[1], latin?.[1]].filter(Boolean).sort((a, b) => a.length - b.length)[0];
  return cut ?? text;
}

function page(lang) {
  const s = STRINGS[lang];
  for (const key of Object.keys(STRINGS.en)) {
    if (!(key in s)) throw new Error(`[ERROR] ${lang} is missing "${key}"`);
  }
  const shotAlt = (name) => {
    const found = s.shots.find(([id]) => id === name);
    if (!found) throw new Error(`[ERROR] no screenshot ${name} in ${lang}`);
    return found[1];
  };

  const alternates = [
    ...ORDER.map((code) => `<link rel="alternate" hreflang="${code}" href="${pageUrl(code)}" />`),
    `<link rel="alternate" hreflang="x-default" href="${pageUrl('en')}" />`,
  ].join('\n');
  const family = FAMILY
    .map(([id, label, url]) => `    <a href="${url}${lang === 'en' ? '' : file(lang)}"${id === STUDIO.id ? ' aria-current="page"' : ''}>${label}</a>`)
    .join('\n');
  const nav = ORDER
    .map((code) => `      <a href="${file(code)}" hreflang="${code}"${code === lang ? ' aria-current="page"' : ''}>${escape(STRINGS[code].label)}</a>`)
    .join('\n');

  const download = `<a class="btn primary" href="${REPO}/releases/latest">${escape(s.ctaDownload)}<small data-release>v${VERSION} · ${STUDIO.installerMB} MB</small></a>`;
  const platforms = `<p class="platforms">${PACKAGES.map(({ id, file: name }) => `<a href="${REPO}/releases/latest/download/${name}" data-package="${id}">${escape(s.platforms[id])}</a>`).join(' · ')}</p>`;

  const tiles = STUDIO.bento.map(({ feature, shot: name }) => {
    const [title, body] = s.features[feature];
    const [width, height] = pictureSize(lang, name);
    return `      <article class="tile">
        <h3>${escape(title)}</h3>
        <figure class="shot"><img src="${shot(lang, name)}" alt="${escape(shotAlt(name))}" loading="lazy" width="${width}" height="${height}" /></figure>
        <p>${escape(firstSentence(body))}</p>
      </article>`;
  }).join('\n');

  // Examples: the page's own language first, the Russian stress pairs next, the other languages folded.
  const clipCard = (clip, extra = '') => `      <figure class="sample${extra}"><figcaption><b>${escape(ENGINES[clip.engine])}</b><span>${escape(voiceName(clip.voice, lang))}<span class="kind">${escape(clip.kind === 'clone' ? s.clone : s.preset)}</span></span></figcaption><audio controls preload="none" src="samples/${clip.file}"></audio></figure>`;
  // a language's name heads its first text only
  const group = (g, at, all) => `    <div class="lang-group" lang="${g.lang}">
${at > 0 && all[at - 1].lang === g.lang ? '' : `      <h3>${escape(s.langNames[g.lang])}</h3>\n`}
      <blockquote>${escape(g.text)}</blockquote>
${g.rows.map((row) => `      <div class="samples">
${row.clips.map((clip) => clipCard(clip)).join('\n')}
      </div>`).join('\n')}
${g.note ? `      <p class="sample-note">${escape(s[g.note])}</p>\n` : ''}    </div>`;
  const own = GROUPS.filter((g) => g.lang === lang);
  const others = GROUPS.filter((g) => g.lang !== lang);
  const stress = `  <section id="stress">
    <h2>${escape(s.stressTitle)}</h2>
    <p class="sub">${escape(s.stressSub)}</p>
${STRESS.map((item) => `    <div class="lang-group" lang="ru">
      <blockquote>${escape(item.text)}</blockquote>
${item.pairs.map((pair) => `      <div class="pair">
${clipCard({ ...pair, file: pair.before }, ' before').replace(`<b>${escape(ENGINES[pair.engine])}</b>`, `<b>${escape(ENGINES[pair.engine])} · ${escape(s.before)}</b>`)}
${clipCard({ ...pair, file: pair.after }, ' after').replace(`<b>${escape(ENGINES[pair.engine])}</b>`, `<b>${escape(ENGINES[pair.engine])} · ${escape(s.after)}</b>`)}
      </div>`).join('\n')}
    </div>`).join('\n')}
  </section>
`;
  const samples = `  <section id="listen">
    <h2>${escape(s.samplesTitle)}</h2>
    <p class="sub">${escape(s.samplesSub)}</p>
${own.map((g, at, all) => group(g, at, all)).join('\n')}
${lang === 'ru' ? '' : `    <details class="more"><summary>${escape(s.samplesMore)} (${others.length})</summary>
${others.map((g, at, all) => group(g, at, all)).join('\n')}
    </details>
`}    <p class="disclaimer">${escape(s.disclaimer)}</p>
  </section>
`;
  // the Russian page leads with its own stress pairs; the others show them after their own examples
  const listen = lang === 'ru' ? `${stress}${samples}` : `${samples}${stress}`;

  const u = s.engineUnits;
  const number = (value) => value.toLocaleString(lang, { minimumFractionDigits: value < 10 ? 1 : 0, maximumFractionDigits: 1 });
  const first = (ms) => (ms == null ? '—' : ms < 1000 ? `${number(ms)} ${u.ms}` : `${number(ms / 1000)} ${u.s}`);
  const gb = (value, approx) => (value == null ? '—' : `${approx ? '~' : ''}${number(value)} ${u.gb}`);
  const times = (value) => (value == null ? '—' : `${number(value)}${u.x}`);
  const engines = `  <section id="engines">
    <h2>${escape(s.enginesTitle)}</h2>
    <p class="sub">${escape(s.enginesSub)}</p>
    <div class="scroll"><table>
      <thead><tr>${s.engineCols.map((col) => `<th>${escape(col)}</th>`).join('')}</tr></thead>
      <tbody>
${ENGINE_BENCH.map((row) => `        <tr><td><b>${escape(ENGINES[row.engine])}</b></td><td>${escape(s.engineRoles[row.engine])}</td><td>${times(row.speed)}</td><td>${first(row.firstMs)}</td><td>${escape(u[row.stream])}</td><td>${gb(row.diskGB)}</td><td>${gb(row.vramGB, row.vramApprox)}</td></tr>`).join('\n')}
      </tbody>
    </table></div>
    <h3>${escape(s.asrTitle)}</h3>
    <p class="sub">${escape(s.asrSub)}</p>
    <div class="scroll"><table>
      <thead><tr>${s.asrCols.map((col) => `<th>${escape(col)}</th>`).join('')}</tr></thead>
      <tbody>
${ASR_BENCH.map((row) => `        <tr><td><b>${escape(row.name)}</b></td><td>${escape(s.asrRoles[row.model])}</td><td>${times(row.gpu)}</td><td>${times(row.cpu)}</td><td>${row.langs}</td><td>${gb(row.diskGB)}</td><td>${gb(row.vramGB)}</td></tr>`).join('\n')}
      </tbody>
    </table></div>
    <h3>${escape(s.liveTitle)}</h3>
    <p class="sub">${escape(s.liveSub)}</p>
    <div class="scroll"><table>
      <tbody>
${LIVE_BENCH.map((row) => `        <tr><td>${escape(s.liveSteps[row.step])}</td><td><b>${row.ms.map(first).join(' – ')}</b></td></tr>`).join('\n')}
      </tbody>
    </table></div>
  </section>
`;

  const shown = new Set([STUDIO.hero, ...STUDIO.bento.map(({ shot: name }) => name)]);
  const capital = (text) => text.charAt(0).toLocaleUpperCase(lang) + text.slice(1);
  const steps = s.steps
    .map(([title, body], index) => `      <li><b><span class="num">${index + 1}</span>${escape(title)}</b><span>${escape(capital(body))}</span></li>`)
    .join('\n');

  // Questions and answers: shown as they are, and handed to search engines and AI answers as FAQPage.
  const faq = [
    { q: s.qGpu, text: [s.gpuAnswer], html: `<p>${escape(s.gpuAnswer)}</p>` },
    { q: s.qFree, text: [s.aFree], html: `<p>${escape(s.aFree)}</p>` },
    { q: s.qOffline, text: [s.privacySub, ...s.privacy.map(([t, b]) => `${t}: ${b}`)], html: `<p>${escape(s.privacySub)}</p><ul>${s.privacy.map(([t, b]) => `<li><b>${escape(t)}</b> — ${escape(b)}</li>`).join('')}</ul>` },
    { q: s.qStress, text: [s.features[STUDIO.faq.stress][1], s.stressSub], html: `<p>${escape(s.features[STUDIO.faq.stress][1])}</p><p>${escape(s.stressSub)}</p>` },
    { q: s.qLicense, text: [s.footerLicense], html: `<p>${escape(s.footerLicense)} <a href="${REPO}#license">README</a></p>` },
    { q: s.qAgent, text: [s.features[STUDIO.faq.mcp][1]], html: `<p>${escape(s.features[STUDIO.faq.mcp][1])}</p><pre>claude mcp add --transport http speech-studio http://127.0.0.1:8797/mcp</pre>` },
    { q: s.qBuilt, text: [s.archSub], html: `<p>${escape(s.archSub)}</p><pre>${escape(STUDIO.arch)}</pre>` },
  ];

  const author = { '@type': 'Person', '@id': 'https://github.com/timoncool#person', name: 'Nerual Dreming', url: 'https://github.com/timoncool',
    sameAs: ['https://github.com/timoncool', 'https://t.me/nerual_dreming', 'https://artgeneration.me'] };
  const ld = {
    '@context': 'https://schema.org',
    '@graph': [
      {
        '@type': 'SoftwareApplication', '@id': `${SITE}#app`, name: NAME, url: pageUrl(lang), inLanguage: lang,
        description: s.description, applicationCategory: 'MultimediaApplication', applicationSubCategory: 'Text to speech and speech recognition',
        operatingSystem: 'Windows 10, Windows 11, macOS 13, Linux', softwareVersion: VERSION, fileSize: `${STUDIO.installerMB} MB`,
        downloadUrl: `${REPO}/releases/latest`, isAccessibleForFree: true, license: 'https://opensource.org/licenses/MIT',
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'USD' },
        image: `${SITE}og-${lang}.jpg`, screenshot: `${SITE}${shot(lang, STUDIO.hero)}`,
        codeRepository: REPO, sameAs: [REPO], author: { '@id': author['@id'] }, dateModified: STUDIO.updated,
      },
      author,
      { '@type': 'FAQPage', inLanguage: lang, mainEntity: faq.map(({ q, text }) => ({ '@type': 'Question', name: q, acceptedAnswer: { '@type': 'Answer', text: text.join(' ') } })) },
    ],
  };

  return `<!doctype html>
<html lang="${lang}">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>${escape(s.title)}</title>
<meta name="description" content="${escape(s.description)}" />
<link rel="canonical" href="${pageUrl(lang)}" />
${alternates}
<link rel="icon" type="image/svg+xml" href="favicon.svg" />
<meta property="og:type" content="website" />
<meta property="og:site_name" content="${NAME}" />
<meta property="og:title" content="${escape(s.title)}" />
<meta property="og:description" content="${escape(s.description)}" />
<meta property="og:url" content="${pageUrl(lang)}" />
<meta property="og:locale" content="${lang}" />
<meta property="og:image" content="${SITE}og-${lang}.jpg" />
<meta property="og:image:width" content="1200" />
<meta property="og:image:height" content="630" />
<meta property="og:image:alt" content="${escape(s.heroTitle)}" />
<meta name="twitter:card" content="summary_large_image" />
<meta name="twitter:image" content="${SITE}og-${lang}.jpg" />
<link rel="preload" as="image" href="${shot(lang, STUDIO.hero)}" fetchpriority="high" />
<script type="application/ld+json">${JSON.stringify(ld).replace(/</g, '\\u003c')}</script>
<style>
${css.trimEnd()}
</style>
</head>
<body>
<header>
  <div class="utility"><div class="wrap utility-bar">
    <nav class="family" aria-label="Studios">
${family}
    </nav>
    <nav class="langs" aria-label="Language">
${nav}
    </nav>
  </div></div>
  <div class="wrap bar">
    <img class="mark" src="favicon.svg" alt="" width="30" height="30" />
    <div class="name">${NAME}</div>
    <a class="head-github" href="${REPO}">GitHub${starsBadge(escape(s.stars))}</a>
    <a class="head-download" href="${REPO}/releases/latest">${escape(s.download)}</a>
  </div>
</header>

<main class="wrap">
  <section class="hero">
    <h1>${escape(s.heroTitle)}</h1>
    <p class="lead">${escape(firstSentence(s.heroLead))}</p>
    <div class="cta">
      ${download}
      <a class="btn" href="${REPO}">${escape(s.starCta)}${starsBadge(escape(s.stars))}</a>
    </div>
    ${platforms}
    <p class="note">${escape(s.ctaNote)}</p>
    <figure class="hero-shot"><img src="${shot(lang, STUDIO.hero)}" alt="${escape(shotAlt(STUDIO.hero))}" width="${pictureSize(lang, STUDIO.hero)[0]}" height="${pictureSize(lang, STUDIO.hero)[1]}" fetchpriority="high" /></figure>
  </section>

${listen}${engines}  <section id="features">
    <h2>${escape(s.featuresTitle)}</h2>
    <div class="bento">
${tiles}
    </div>
  </section>

  <section id="screenshots">
    <h2>${escape(s.shotsTitle)}</h2>
    <p class="sub">${escape(s.shotsSub)}</p>
    <div class="gallery">
${s.shots.filter(([name]) => !shown.has(name)).map(([name, caption]) => {
    const [width, height] = pictureSize(lang, name);
    return `      <figure><img src="${shot(lang, name)}" alt="${escape(caption)}" loading="lazy" width="${width}" height="${height}" /><figcaption>${escape(caption)}</figcaption></figure>`;
  }).join('\n')}
    </div>
  </section>

  <section id="start">
    <h2>${escape(s.startTitle)}</h2>
    <ol class="steps">
${steps}
    </ol>
    <div class="cta">${download}</div>
  </section>

  <section id="faq">
    <h2>FAQ</h2>
${faq.map(({ q, html }) => `    <details><summary>${escape(q)}</summary><div class="answer">${html}</div></details>`).join('\n')}
  </section>

  <section id="author">
    <h2>${escape(s.authorTitle)}</h2>
    <p class="sub">${escape(s.authorSub)}</p>
    <div class="pills">
      <a class="pill" href="https://github.com/timoncool">GitHub · @timoncool</a>
      <a class="pill" href="https://t.me/nerual_dreming">Telegram · @nerual_dreming</a>
      <a class="pill" href="https://t.me/neuroport">Telegram · @neuroport</a>
      <a class="pill" href="https://artgeneration.me">ArtGeneration.me</a>
      <a class="pill" href="donate.html?lang=${lang}">${escape(s.ctaDonate)}</a>
    </div>
  </section>

  <footer>
    <p>${escape(s.footerLicense)}</p>
    <p><a href="${REPO}/blob/main/CHANGELOG.md">${escape(s.footerChanges)}</a> · <a href="${REPO}/issues">${escape(s.footerIssues)}</a> · ${escape(s.updated)} <time datetime="${STUDIO.updated}">${STUDIO.updated}</time></p>
  </footer>
</main>

<div class="viewer" id="viewer" aria-hidden="true">
  <button type="button" aria-label="Close">×</button>
  <img id="viewer-image" alt="" />
</div>
<script>
const viewer = document.getElementById('viewer');
const viewerImage = document.getElementById('viewer-image');
const closeViewer = () => { viewer.classList.remove('open'); viewer.setAttribute('aria-hidden', 'true'); };
document.addEventListener('click', (event) => {
  const image = event.target.closest('.hero-shot img, .tile img, .gallery img');
  if (image) {
    viewerImage.src = image.src;
    viewerImage.alt = image.alt;
    viewer.classList.add('open');
    viewer.setAttribute('aria-hidden', 'false');
    return;
  }
  if (event.target.closest('#viewer')) closeViewer();
});
document.addEventListener('keydown', (event) => { if (event.key === 'Escape') closeViewer(); });
// one example plays at a time
document.addEventListener('play', (event) => {
  document.querySelectorAll('audio').forEach((audio) => { if (audio !== event.target) audio.pause(); });
}, true);
// the download buttons name the latest release as GitHub has it, so a new release needs no new page
fetch('https://api.github.com/repos/${REPO.replace('https://github.com/', '')}/releases/latest')
  .then((response) => (response.ok ? response.json() : Promise.reject(new Error('GitHub answered ' + response.status))))
  .then((release) => {
    const installer = release.assets.find((asset) => asset.name.endsWith('-setup.exe'));
    if (!installer) throw new Error('the latest release has no installer');
    const label = release.tag_name + ' · ' + Math.round(installer.size / 1048576) + ' MB';
    document.querySelectorAll('[data-release]').forEach((element) => { element.textContent = label; });
    const ends = ${JSON.stringify(Object.fromEntries(PACKAGES.map(({ id, ends }) => [id, ends])))};
    document.querySelectorAll('[data-package]').forEach((link) => {
      const asset = release.assets.find((item) => item.name.endsWith(ends[link.dataset.package]));
      link.href = asset ? asset.browser_download_url : release.html_url;
    });
  })
  .catch((error) => console.warn('Latest release not read; the page shows the one it was built with:', error));
</script>
</body>
</html>
`;
}

// The donation content picks its own language (?lang=, the reader's choice, the browser), so one page serves all.
function donatePage() {
  const family = FAMILY
    .map(([id, label, url]) => `    <a href="${url}"${id === STUDIO.id ? ' aria-current="page"' : ''}>${label}</a>`)
    .join('\n');
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>Support the work · ${NAME}</title>
<link rel="canonical" href="${SITE}donate.html" />
<link rel="icon" type="image/svg+xml" href="favicon.svg" />
<style>
${css.trimEnd()}
.donate,.donate-lang{--bg2:var(--bg);--surface:var(--panel);--surface2:var(--panel-2);--line2:rgba(255,255,255,.14);--ink:var(--text);--body:#c4c4cc;--mut:var(--muted);--faint:#6b6b75;--accent-ink:var(--accent);--on-accent:#041019;--accent-soft:rgba(76,194,255,.12);--accent-line:rgba(76,194,255,.34);--r:16px;--r2:12px;--r3:8px;--fs-s:13px;--fs-m:15px;--btn:44px;--btn-s:36px;--maxw:900px;--ease:cubic-bezier(.33,1,.68,1);--font:-apple-system,"Segoe UI",Roboto,"Helvetica Neue",Arial,"Noto Sans",sans-serif}
${DONATE['donate.css']}
.donate .hero .apps{text-align:left}
</style>
</head>
<body data-hub-app="${STUDIO.id}">
<header>
  <div class="utility"><div class="wrap utility-bar">
    <nav class="family" aria-label="Studios">
${family}
    </nav>
    <div class="donate-lang">
${DONATE['donate-lang.html']}
    </div>
  </div></div>
  <div class="wrap bar">
    <img class="mark" src="favicon.svg" alt="" width="30" height="30" />
    <div class="name">${NAME}</div>
    <a class="head-github" href="${REPO}">GitHub${starsBadge('GitHub stars')}</a>
  </div>
</header>

<main class="wrap donate">
${DONATE['donate-body.html']}
</main>
</body>
</html>
`;
}

for (const lang of ORDER) {
  writeFileSync(join(docs, file(lang)), page(lang), 'utf8');
  console.log(`[OK] docs/${file(lang)}`);
}
writeFileSync(join(docs, 'donate.html'), donatePage(), 'utf8');
console.log('[OK] docs/donate.html');

// The sitemap names every language of the page and ties them together, as the pages do.
const links = [...ORDER.map((code) => `    <xhtml:link rel="alternate" hreflang="${code}" href="${pageUrl(code)}"/>`),
  `    <xhtml:link rel="alternate" hreflang="x-default" href="${pageUrl('en')}"/>`].join('\n');
const sitemap = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9" xmlns:xhtml="http://www.w3.org/1999/xhtml">
${ORDER.map((code) => `  <url>
    <loc>${pageUrl(code)}</loc>
    <lastmod>${STUDIO.updated}</lastmod>
${links}
  </url>`).join('\n')}
</urlset>
`;
writeFileSync(join(docs, 'sitemap.xml'), sitemap, 'utf8');
console.log('[OK] docs/sitemap.xml');
