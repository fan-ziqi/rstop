'use strict';
document.documentElement.classList.add('js');
const views = [
  ['overview', 'Overview'], ['processes', 'Processes'], ['gpu', 'GPU'], ['storage', 'Storage'],
  ['network', 'Network'], ['system', 'System'], ['containers', 'Containers'], ['services', 'Services']
];
let currentView = 0;
let currentTheme = 'midnight';
const tabs = [...document.querySelectorAll('[data-view]')];
function showView(index, focus = false) {
  currentView = (index + views.length) % views.length;
  const [key, label] = views[currentView];
  tabs.forEach((tab, i) => {
    tab.setAttribute('aria-selected', String(i === currentView));
    tab.tabIndex = i === currentView ? 0 : -1;
  });
  const image = document.querySelector('#demo-image');
  image.src = `assets/${key}-${currentTheme}.svg`;
  image.alt = `rstop ${label} with illustrative data`;
  document.querySelector('#demo-panel').setAttribute('aria-labelledby', `tab-${key}`);
  document.querySelector('#view-index').textContent = `${String(currentView + 1).padStart(2, '0')} / 08`;
  if (focus) tabs[currentView].focus({ preventScroll: true });
}
tabs.forEach((tab, i) => tab.addEventListener('click', () => showView(i)));
document.querySelector('#demo').addEventListener('keydown', event => {
  let next;
  if (/^[1-8]$/.test(event.key)) next = Number(event.key) - 1;
  else if (event.key === 'ArrowRight') next = currentView + 1;
  else if (event.key === 'ArrowLeft') next = currentView - 1;
  else if (event.key === 'Home') next = 0;
  else if (event.key === 'End') next = 7;
  if (next !== undefined) { event.preventDefault(); showView(next, true); }
});
document.querySelectorAll('[data-theme]').forEach(button => button.addEventListener('click', () => {
  currentTheme = button.dataset.theme;
  document.querySelectorAll('[data-theme]').forEach(b => b.setAttribute('aria-pressed', String(b === button)));
  document.querySelector('#theme-image').src = `assets/overview-${currentTheme}.svg`;
  document.querySelector('#theme-image').alt = `${currentTheme} theme with illustrative data`;
  document.querySelector('#theme-command').textContent = `rstop --theme ${currentTheme}`;
  showView(currentView);
}));
const commands = {
  unix: "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/fan-ziqi/rstop/releases/latest/download/rstop-installer.sh | sh",
  windows: 'powershell -ExecutionPolicy Bypass -c "irm https://github.com/fan-ziqi/rstop/releases/latest/download/rstop-installer.ps1 | iex"'
};
let currentOS = 'unix';
document.querySelectorAll('[data-os]').forEach(button => button.addEventListener('click', () => {
  currentOS = button.dataset.os;
  document.querySelectorAll('[data-os]').forEach(b => b.setAttribute('aria-pressed', String(b === button)));
  document.querySelector('#install-command').textContent = commands[currentOS];
  document.querySelector('#copy-status').textContent = '';
}));
let copyTimer;
document.querySelector('#copy-command').addEventListener('click', async () => {
  const status = document.querySelector('#copy-status');
  try { await navigator.clipboard.writeText(commands[currentOS]); status.textContent = 'Copied.'; }
  catch {
    const range = document.createRange(); range.selectNodeContents(document.querySelector('#install-command'));
    const selection = window.getSelection(); selection.removeAllRanges(); selection.addRange(range);
    status.textContent = 'Press Ctrl+C (macOS: ⌘C) to copy.';
  }
  clearTimeout(copyTimer);
  copyTimer = setTimeout(() => { status.textContent = ''; }, 4000);
});
if ('IntersectionObserver' in window) {
  const observer = new IntersectionObserver(entries => entries.forEach(entry => {
    if (entry.isIntersecting) { entry.target.classList.add('visible'); observer.unobserve(entry.target); }
  }), { threshold: .06 });
  document.querySelectorAll('.reveal').forEach(element => observer.observe(element));
} else document.querySelectorAll('.reveal').forEach(element => element.classList.add('visible'));
