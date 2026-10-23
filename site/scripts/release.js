// Shared by deploy stamping and the optional browser refresh. Never infer an
// asset from a tag: optional formats and architectures can be missing.
export const RELEASE_API = "https://api.github.com/repos/worxbend/scenedeck/releases/latest";
const RELEASES = "https://github.com/worxbend/scenedeck/releases";
const FORMATS = { binary: "", tar: ".tar.gz", appimage: ".AppImage", flatpak: ".flatpak" };

export function normalizeRelease(raw) {
  if (!/^v\d+\.\d+\.\d+$/.test(raw?.tag_name) || !Array.isArray(raw.assets) || raw.draft || raw.prerelease) {
    throw new Error("Expected a published vX.Y.Z release with an asset inventory");
  }
  const tag = raw.tag_name;
  const version = tag.slice(1);
  const assets = {};
  for (const [format, suffix] of Object.entries(FORMATS)) {
    for (const arch of ["amd64", "arm64"]) {
      const name = `scenedeck-${version}-linux-${arch}${suffix}`;
      const url = `${RELEASES}/download/${tag}/${name}`;
      if (raw.assets.some((asset) => asset.name === name && asset.browser_download_url === url)) {
        assets[`${format}-${arch}`] = url;
      }
    }
  }
  return { tag, version, url: `${RELEASES}/tag/${tag}`, assets };
}

function commands(release) {
  const file = (key) => release.assets[key]?.split("/").pop();
  const appimage = file("appimage-amd64");
  const binary = file("binary-amd64");
  const flatpak = file("flatpak-amd64");
  return {
    "appimage-chmod": appimage ? `chmod +x ${appimage}` : "",
    "appimage-run": appimage ? `./${appimage}` : "",
    "binary-run": binary ? `chmod +x ${binary} && ./${binary}` : "",
    "flatpak-install": flatpak ? `flatpak install --user ./${flatpak}` : "",
  };
}

function unavailable(release, format) {
  return !Object.keys(release.assets).some((key) => key.startsWith(`${format}-`));
}

export function applyRelease(document, release) {
  const command = commands(release);
  document.querySelectorAll("[data-release-version]").forEach((el) => { el.textContent = release.tag; });
  document.querySelectorAll("[data-release-page]").forEach((el) => { el.href = release.url; });
  document.querySelectorAll("[data-release-asset]").forEach((el) => {
    const url = release.assets[el.dataset.releaseAsset];
    el.hidden = !url;
    if (url) el.setAttribute("href", url);
    else el.removeAttribute("href");
  });
  document.querySelectorAll("[data-release-requires]").forEach((el) => {
    el.hidden = !release.assets[el.dataset.releaseRequires];
  });
  document.querySelectorAll("[data-release-unavailable]").forEach((el) => {
    el.hidden = !unavailable(release, el.dataset.releaseUnavailable);
  });
  document.querySelectorAll("[data-release-command]").forEach((el) => {
    // Preserve the coloured prompt, but replace only the command text.
    el.lastChild.nodeValue = ` ${command[el.dataset.releaseCommand]}`;
  });
  document.querySelectorAll("[data-release-copy]").forEach((el) => {
    el.setAttribute("data-copy", command[el.dataset.releaseCopy]);
  });
  const json = document.querySelector('script[type="application/ld+json"]');
  if (json) {
    const metadata = JSON.parse(json.textContent);
    metadata.softwareVersion = release.version;
    metadata.downloadUrl = release.url;
    json.textContent = JSON.stringify(metadata, null, 2);
  }
}

const escapeHTML = (text) => text.replaceAll("&", "&amp;").replaceAll('"', "&quot;").replaceAll("<", "&lt;");
const attribute = (tag, name) => tag.match(new RegExp(`\\b${name}="([^"]*)"`))?.[1];
function setAttribute(tag, name, value) {
  tag = tag.replace(new RegExp(` ${name}(?:="[^"]*")?(?=[\\s>])`, "g"), "");
  return value === null ? tag : tag.replace(/>$/, ` ${name}${value === "" ? "" : `="${escapeHTML(value)}"`}>`);
}

// Only our explicit markers are rewritten; no broad filename substitution.
export function renderReleaseHTML(html, release) {
  const command = commands(release);
  html = html.replace(/<[^!][^>]*\bdata-release-[^>]*>/g, (tag) => {
    const asset = attribute(tag, "data-release-asset");
    if (asset) {
      tag = setAttribute(tag, "href", release.assets[asset] ?? null);
      tag = setAttribute(tag, "hidden", release.assets[asset] ? null : "");
    }
    const required = attribute(tag, "data-release-requires");
    if (required) tag = setAttribute(tag, "hidden", release.assets[required] ? null : "");
    const format = attribute(tag, "data-release-unavailable");
    if (format) tag = setAttribute(tag, "hidden", unavailable(release, format) ? null : "");
    const copy = attribute(tag, "data-release-copy");
    if (copy) tag = setAttribute(tag, "data-copy", command[copy]);
    if (tag.includes("data-release-page")) tag = setAttribute(tag, "href", release.url);
    return tag;
  });
  html = html.replace(/(<[^>]+\bdata-release-version[^>]*>)[^<]*/g, `$1${release.tag}`);
  html = html.replace(/(<code\b[^>]*data-release-command="([^"]+)"[^>]*><span[^>]*>\$<\/span>)[^<]*/g,
    (_, prompt, key) => `${prompt} ${escapeHTML(command[key])}`);
  return html.replace(/("softwareVersion"\s*:\s*")[^"]*(")/, `$1${release.version}$2`)
    .replace(/("downloadUrl"\s*:\s*")[^"]*(")/, `$1${release.url}$2`);
}
