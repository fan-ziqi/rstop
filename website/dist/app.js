'use strict';
document.documentElement.classList.add('js');
const themeButtons = [...document.querySelectorAll('[data-theme]')];
const resourceDemo = document.querySelector('#resource-demo');
const install = document.querySelector('#install');
resourceDemo.addEventListener('native-theme-change', ({detail}) => {
  install.dataset.theme = detail.theme;
  Object.entries(detail.colors).forEach(([name,color]) => install.style.setProperty(`--install-${name}`,color));
});
themeButtons.forEach(button => button.addEventListener('click', () => {
  resourceDemo.setAttribute('theme', button.dataset.theme);
  themeButtons.forEach(item => item.setAttribute('aria-pressed', String(item === button)));
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
