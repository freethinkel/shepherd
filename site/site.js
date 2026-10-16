const repo = 'https://api.github.com/repos/freethinkel/shepherd'

// The hero button offers this machine's build; the row's own button is the fallback link until the API answers.
const ua = navigator.userAgent
const os = /Mac|iPhone|iPad/.test(ua) ? 'macos' : /Windows/.test(ua) ? 'windows' : /Linux|X11/.test(ua) ? 'linux' : null
const hero = document.getElementById('hero-download')
if (os) {
  hero.dataset.downloadOs = os
  hero.textContent = 'Download for ' + { macos: 'macOS', windows: 'Windows', linux: 'Linux' }[os]
  hero.href = document.querySelector(`[data-os="${os}"]`).href
}

// Point each download at the newest release that has its file; without the API the links open the releases page.
fetch(repo + '/releases?per_page=10')
  .then(r => r.ok ? r.json() : [])
  .then(releases => {
    releases = releases.filter(r => !r.draft && !r.prerelease)
    if (!releases.length) return
    const v = document.querySelector('.version')
    v.textContent = 'Latest release: ' + releases[0].tag_name + '.'
    v.hidden = false
    for (const a of document.querySelectorAll('[data-asset]')) {
      const re = new RegExp(a.dataset.asset)
      for (const r of releases) {
        const f = r.assets.find(f => re.test(f.name))
        if (f) { a.href = f.browser_download_url; a.title = f.name + ', ' + Math.round(f.size / 1e6) + ' MB'; break }
      }
    }
    if (os) hero.href = document.querySelector(`[data-os="${os}"]`).href
  })
  .catch(() => {})

fetch(repo)
  .then(r => r.ok ? r.json() : null)
  .then(j => {
    if (!j || !j.stargazers_count) return
    const s = document.querySelector('.gh-stars')
    s.textContent = '★ ' + (j.stargazers_count >= 1000 ? (j.stargazers_count / 1000).toFixed(1) + 'k' : j.stargazers_count)
    s.hidden = false
  })
  .catch(() => {})

// The hero's glyph field: dim terminal words scattered on a cell grid, drawn once at the hero's size.
// ponytail: static; Zeron's drifts under the pointer, add if the hero feels flat.
const field = document.querySelector('.glyphs')
const words = 'herdr claude codex pane split tab approve waiting done idle git worktree main pr ci ⌘K ssh agent blocked ⌥⌘B ⇧⌘A cargo make run'.split(' ')
function drawField() {
  const dpr = devicePixelRatio || 1
  const w = field.clientWidth, h = field.clientHeight
  field.width = w * dpr; field.height = h * dpr
  const c = field.getContext('2d')
  c.scale(dpr, dpr)
  c.font = '10px "Geist Mono", ui-monospace, monospace'
  c.fillStyle = '#7b879c'
  const cw = 7, ch = 24
  let seed = 7
  const rnd = () => (seed = (seed * 16807) % 2147483647) / 2147483647
  for (let y = ch; y < h; y += ch) {
    for (let x = 0; x < w;) {
      if (rnd() < 0.1) {
        const word = words[Math.floor(rnd() * words.length)]
        c.globalAlpha = 0.08 + rnd() * 0.22
        c.fillText(word, x, y)
        x += (word.length + 3) * cw
      } else x += cw * (2 + Math.floor(rnd() * 5))
    }
  }
}
document.fonts ? document.fonts.ready.then(drawField) : drawField()
addEventListener('resize', drawField)
