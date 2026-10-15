import { el, q, setText } from './dom';
import { icon } from './icons';
import { t } from '../i18n';

export class Dialog {
  readonly scrim: HTMLElement;
  readonly body: HTMLElement;
  private readonly foot: HTMLElement;
  private readonly before: Element | null;

  constructor(title: string, size: 'normal' | 'wide' = 'normal') {
    this.before = document.activeElement;
    this.scrim = el(`<div class="cv-scrim" hidden><div class="cv-dialog${size === 'wide' ? ' is-wide' : ''}" role="dialog" aria-modal="true">
<header class="cv-dialog-head"><h2 class="cv-dialog-title"></h2><button type="button" class="icon-btn cv-dialog-x" aria-label="Close">${icon('close', 16)}</button></header>
<div class="cv-dialog-body"></div>
<footer class="cv-dialog-foot"></footer>
</div></div>`);
    setText(q(this.scrim, '.cv-dialog-title'), t(title));
    this.body = q(this.scrim, '.cv-dialog-body');
    this.foot = q(this.scrim, '.cv-dialog-foot');
    q(this.scrim, '.cv-dialog-x').addEventListener('click', () => this.close());
    this.scrim.addEventListener('pointerdown', (e) => { if (e.target === this.scrim) this.close(); });
    this.scrim.addEventListener('keydown', (e) => { if (e.key === 'Escape') { e.stopPropagation(); this.close(); } });
    document.getElementById('overlay-root')?.appendChild(this.scrim);
  }

  action(label: string, run: () => void | Promise<void>, primary: boolean): void {
    const b = el<HTMLButtonElement>(`<button type="button" class="btn${primary ? ' primary' : ''}"></button>`);
    setText(b, t(label));
    b.addEventListener('click', () => void run());
    this.foot.appendChild(b);
  }

  open(): void {
    this.scrim.hidden = false;
    const first = this.scrim.querySelector<HTMLElement>('select, input, .cv-dialog-foot .primary');
    first?.focus();
  }

  close(): void {
    this.scrim.remove();
    if (this.before instanceof HTMLElement) this.before.focus();
  }
}
