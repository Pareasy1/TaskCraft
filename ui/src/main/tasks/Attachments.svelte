<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { Ellipsis, ExternalLink, FileText, FolderOpen, Link, Paperclip, Plus } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage, knownErrorMessage, onItemsChanged } from '../../lib/api';
  import { formatSize, isWebUrl, notAddedMessage, urlHost } from '../../lib/attachments';
  import { PopupMenu } from '../../lib/menu.svelte';
  import { dropIndex, insertAt } from '../../lib/plans';
  import { restoreRemovedAttachment } from '../../lib/restore';
  import { focusAfterRemoval, shiftId } from '../../lib/subtasks';
  import { notify } from '../../lib/toast.svelte';
  import type { Attachment, Task } from '../../lib/types';
  import Menu from '../Menu.svelte';

  let { task, onerror }: { task: Task; onerror: (m: string) => void } = $props();

  let items = $state<Attachment[]>([]);
  let listEl = $state<HTMLUListElement | undefined>();
  let fileBtn = $state<HTMLButtonElement | undefined>();

  async function load() {
    const id = task.id;
    try {
      const list = await api.listAttachments(id);
      if (task.id === id) items = list;
    } catch (e) {
      onerror(errorMessage(e));
    }
  }
  $effect(() => {
    void task.id;
    load();
  });
  const unlisten = onItemsChanged(() => load());
  onDestroy(() => void unlisten.then((f) => f()));

  /** Runs a store command; a failure shows the error and reloads (snapping a reorder back). */
  async function run(fn: () => Promise<unknown>) {
    try {
      await fn();
    } catch (e) {
      onerror(errorMessage(e));
      load();
    }
  }

  const menuBtn = (id: number) => listEl?.querySelector<HTMLElement>(`[data-att-menu="${id}"]`) ?? undefined;
  /** Size for a file; host for a link, unless the name already is the host. */
  const detail = (a: Attachment) => {
    if (a.kind === 'file') return a.size != null ? formatSize(a.size) : '';
    const host = urlHost(a.url ?? '');
    return host === a.name ? '' : host;
  };

  // ── Open / Show in folder: a stored file that has gone offers to remove the row ──
  async function reach(a: Attachment, fn: () => Promise<void>) {
    try {
      await fn();
    } catch (e) {
      const code = e && typeof e === 'object' && 'code' in e ? (e as { code: unknown }).code : null;
      if (code !== 'not_found' || a.kind !== 'file') return onerror(errorMessage(e));
      // Nothing to bring back, so this remove has no Undo.
      notify(t('attachments.missing'), undefined, { label: t('attachments.remove'), run: () => api.removeAttachment(a.id).then(() => {}) });
    }
  }
  const open = (a: Attachment) => reach(a, () => api.openAttachment(a.id));
  const reveal = (a: Attachment) => reach(a, () => api.revealAttachment(a.id));

  // ── Rename inline: Enter or leaving the field saves, Esc cancels ──
  let renaming = $state<number | null>(null);
  async function startRename(a: Attachment) {
    renaming = a.id;
    await tick();
    const el = listEl?.querySelector<HTMLInputElement>(`[data-att-name="${a.id}"]`);
    el?.focus();
    el?.select();
  }
  function endRename(el: HTMLInputElement, a: Attachment, save: boolean) {
    if (renaming !== a.id) return;
    renaming = null;
    const name = el.value.trim();
    if (save && name && name !== a.name) run(() => api.renameAttachment(a.id, name));
    tick().then(() => menuBtn(a.id)?.focus());
  }
  function renameKeys(e: KeyboardEvent, a: Attachment) {
    const el = e.currentTarget as HTMLInputElement;
    if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) endRename(el, a, true);
    else if (e.key === 'Escape') {
      e.stopPropagation(); // cancels the rename; doesn't close the dialog
      endRename(el, a, false);
    } else return;
    e.preventDefault();
  }

  // ── Order ──
  let refocus: number | null = null;
  $effect(() => {
    void items;
    if (refocus == null) return;
    const id = refocus;
    refocus = null;
    tick().then(() => menuBtn(id)?.focus());
  });
  function shift(a: Attachment, by: -1 | 1) {
    const next = shiftId(items.map((x) => x.id), a.id, by);
    if (!next) return;
    refocus = a.id;
    run(() => api.reorderAttachments(task.id, next));
  }

  // Drag is the mouse shortcut; the row menu's Move up/down is the keyboard path.
  let dragId = $state<number | null>(null);
  let dropAt = $state<number | null>(null);
  function dragOver(e: DragEvent) {
    if (dragId == null) return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    const rows = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[data-att]')].filter((el) => Number(el.dataset.att) !== dragId);
    dropAt = dropIndex(rows.map((el) => { const r = el.getBoundingClientRect(); return r.top + r.height / 2; }), e.clientY);
  }
  function drop(e: DragEvent) {
    e.preventDefault();
    const id = dragId, at = dropAt;
    endDrag();
    if (id == null || at == null) return;
    const ids = items.map((x) => x.id);
    // `at` counts the rows other than the dragged one; insertAt counts the full list.
    const next = insertAt(ids, id, at > ids.indexOf(id) ? at + 1 : at);
    if (next.some((x, i) => x !== ids[i])) run(() => api.reorderAttachments(task.id, next));
  }
  function endDrag() {
    dragId = dropAt = null;
  }
  const restIndex = (a: Attachment) => items.filter((x) => x.id !== dragId).indexOf(a);

  // ── Remove (with Undo: the stored file waits long enough to be re-linked) ──
  async function remove(a: Attachment) {
    const order = items.map((x) => x.id);
    const next = focusAfterRemoval(order, a.id);
    try {
      await api.removeAttachment(a.id);
    } catch (e) {
      onerror(errorMessage(e));
      menuBtn(a.id)?.focus();
      return;
    }
    // The neighbour's row stays when the list reloads (keyed), so it can take focus now.
    ((next == null ? null : menuBtn(next)) ?? fileBtn)?.focus();
    notify(t('attachments.removed', { name: a.name }), () => restoreRemovedAttachment(a, order));
  }

  // ── Adding ──
  let picking = $state(false);
  async function addFiles() {
    if (picking) return;
    picking = true;
    try {
      const { errors } = await api.pickAndAddFiles(task.id);
      const msg = notAddedMessage(errors, knownErrorMessage, (names, reason) => t('attachments.notAdded', { names, reason }), t('attachments.copyFailed'));
      if (msg) notify(msg);
    } catch (e) {
      onerror(errorMessage(e));
    } finally {
      picking = false;
    }
  }

  let linkOpen = $state(false);
  let url = $state('');
  let urlError = $state(false);
  let urlEl = $state<HTMLInputElement | undefined>();
  let pending: Promise<void> = Promise.resolve();
  async function showLink() {
    linkOpen = true;
    await tick();
    urlEl?.focus();
  }
  /** Closes the link field; `refocus` (the Esc path) moves focus to "Add file…" instead of <body>. */
  function hideLink(refocus = false) {
    linkOpen = false;
    url = '';
    urlError = false;
    if (refocus) tick().then(() => fileBtn?.focus());
  }
  function addLink(): Promise<void> {
    const value = url.trim();
    if (!value) return pending;
    if (!isWebUrl(value)) {
      urlError = true;
      // Save/Close wait on this field: show it (the dialog may be scrolled) so Esc can clear it.
      urlEl?.focus();
      tick().then(() => (document.getElementById(`att-url-error-${task.id}`) ?? urlEl)?.scrollIntoView({ block: 'nearest' }));
      return Promise.reject(new Error('attachment_url'));
    }
    url = ''; // the field keeps focus for the next one
    urlError = false;
    const job = api.addLinkAttachment(task.id, value).then(
      () => {},
      (e) => {
        if (!url) url = value; // give the typed text back
        onerror(errorMessage(e));
        throw e;
      },
    );
    pending = pending.then(() => job).catch(() => {});
    return job;
  }
  /** Adds a typed but not yet added link (the dialog calls this before saving or closing). */
  export async function flush(): Promise<void> {
    await pending;
    await addLink();
  }
  function linkKeys(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.metaKey && !e.ctrlKey) {
      e.preventDefault();
      addLink().catch(() => {});
    } else if (e.key === 'Escape') {
      e.stopPropagation(); // clears the field (or closes it when empty); doesn't close the dialog
      e.preventDefault();
      if (url) {
        url = '';
        urlError = false;
      } else hideLink(true);
    }
  }

  const menu = new PopupMenu<Attachment>();
</script>

<section class="attachments" aria-label={t('attachments.title')}>
  <h3>{t('attachments.title')}{#if items.length}<span class="n">{items.length}</span>{/if}</h3>
  {#if items.length}
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions (drag is a mouse shortcut; the row menu is the keyboard path) -->
    <ul bind:this={listEl} ondragover={dragOver} ondrop={drop}
      ondragleave={(e) => { if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node)) dropAt = null; }}
      class:drop-end={dragId != null && dropAt === items.length - 1}>
      {#each items as a (a.id)}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions (drag and double-click are mouse shortcuts; the row buttons are the keyboard path) -->
        <li
          data-att={a.id}
          class:dragging={dragId === a.id}
          class:drop-before={dragId != null && dragId !== a.id && dropAt === restIndex(a)}
          draggable={renaming !== a.id}
          ondragstart={(e) => { dragId = a.id; e.dataTransfer?.setData('text/plain', a.name); if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move'; }}
          ondragend={endDrag}
          ondblclick={(e) => { if (renaming !== a.id && !(e.target as Element).closest('button, input')) open(a); }}
          oncontextmenu={(e) => { e.preventDefault(); menu.open(a, e.clientX, e.clientY, menuBtn(a.id) ?? (e.currentTarget as HTMLElement)); }}
        >
          <span class="kind" aria-hidden="true">
            {#if a.kind === 'file'}<FileText size={14} />{:else}<Link size={14} />{/if}
          </span>
          {#if renaming === a.id}
            <input class="name-edit" data-att-name={a.id} value={a.name} maxlength="120" aria-label={t('attachments.nameOf', { name: a.name })}
              onkeydown={(e) => renameKeys(e, a)} onblur={(e) => endRename(e.currentTarget, a, true)} />
          {:else}
            <span class="name" title={a.kind === 'link' ? a.url : a.name}>{a.name}</span>
            <span class="detail">{detail(a)}</span>
          {/if}
          <button class="icon-btn act" aria-label={t('attachments.openNamed', { name: a.name })} title={t('attachments.open')}
            onclick={() => open(a)}><ExternalLink size={13} aria-hidden="true" /></button>
          {#if a.kind === 'file'}
            <button class="icon-btn act" aria-label={t('attachments.revealNamed', { name: a.name })} title={t('attachments.reveal')}
              onclick={() => reveal(a)}><FolderOpen size={13} aria-hidden="true" /></button>
          {/if}
          <button class="icon-btn act" data-att-menu={a.id} class:open={menu.current?.target.id === a.id}
            aria-label={t('attachments.options', { name: a.name })} title={t('attachments.options', { name: a.name })}
            aria-haspopup="menu" aria-expanded={menu.current?.target.id === a.id}
            onclick={(e) => menu.openBelow(a, e.currentTarget, (y) => y.id === a.id)}
            onkeydown={(e) => menu.openFromKey(e, a)}><Ellipsis size={14} aria-hidden="true" /></button>
        </li>
      {/each}
    </ul>
  {/if}
  {#if linkOpen}
    <div class="link-row">
      <span class="kind" aria-hidden="true"><Link size={13} /></span>
      <input bind:this={urlEl} bind:value={url} type="url" placeholder={t('attachments.linkPlaceholder')} aria-label={t('attachments.linkLabel')}
        aria-invalid={urlError} aria-describedby={urlError ? `att-url-error-${task.id}` : undefined} maxlength="2000"
        oninput={() => (urlError = false)} onkeydown={linkKeys} onblur={() => { if (!url.trim()) hideLink(); }} />
    </div>
    {#if urlError}<p class="inline-error" id="att-url-error-{task.id}">{t('error.attachment_url')}</p>{/if}
  {/if}
  <div class="adds">
    <button class="add" bind:this={fileBtn} onclick={addFiles} disabled={picking}><Paperclip size={13} aria-hidden="true" />{t('attachments.addFile')}</button>
    {#if !linkOpen}
      <button class="add" onclick={showLink}><Plus size={13} aria-hidden="true" />{t('attachments.addLink')}</button>
    {/if}
  </div>
</section>

{#if menu.current}
  {@const a = menu.current.target}
  {@const i = items.findIndex((x) => x.id === a.id)}
  <Menu {menu} label={a.name}>
    <button role="menuitem" onclick={() => menu.act(open)}>{t('attachments.open')}</button>
    {#if a.kind === 'file'}
      <button role="menuitem" onclick={() => menu.act(reveal)}>{t('attachments.reveal')}</button>
    {/if}
    <button role="menuitem" onclick={() => menu.act(startRename, false)}>{t('attachments.rename')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" disabled={i <= 0} onclick={() => menu.act((y) => shift(y, -1))}>{t('task.moveUp')}</button>
    <button role="menuitem" disabled={i < 0 || i >= items.length - 1} onclick={() => menu.act((y) => shift(y, 1))}>{t('task.moveDown')}</button>
    <div class="sep" role="separator"></div>
    <button role="menuitem" class="danger" onclick={() => menu.act(remove, false)}>{t('attachments.remove')}</button>
  </Menu>
{/if}

<style>
  .attachments { display: flex; flex-direction: column; gap: 4px; }
  h3 { margin: 4px 0 2px; font-size: 12px; font-weight: 600; color: var(--muted); display: flex; gap: 8px; align-items: baseline; }
  .n { font-weight: 400; color: var(--faint); font-variant-numeric: tabular-nums; }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  li { position: relative; display: flex; align-items: center; gap: 8px; padding: 2px 6px; border-radius: 7px; min-height: 30px; cursor: grab; }
  li:hover { background: var(--hover); }
  li.dragging { opacity: 0.45; }
  li.drop-before::before, ul.drop-end::after {
    content: '';
    display: block;
    height: 2px;
    border-radius: 1px;
    background: var(--accent);
  }
  li.drop-before::before { position: absolute; left: 4px; right: 4px; top: -1px; }
  ul.drop-end::after { margin: 0 4px; }
  .kind { display: inline-flex; flex: none; color: var(--muted); }
  .name { flex: 0 1 auto; min-width: 0; font-size: 13px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .detail { flex: 1 0 auto; font-size: 11.5px; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }
  .name-edit { flex: 1; min-width: 0; border: 1px solid var(--accent); background: var(--bg); border-radius: 5px; padding: 3px 5px; margin-left: -5px; font-size: 13px; color: var(--text); outline: none; box-shadow: 0 0 0 3px var(--accent-soft); }
  .act { padding: 2px; opacity: 0; flex: none; }
  li:hover .act, li:focus-within .act, .act:focus-visible, .act.open { opacity: 1; }
  .link-row { display: flex; align-items: center; gap: 8px; padding: 2px 6px; }
  .link-row input { flex: 1; min-width: 0; border: 1px solid var(--border); background: var(--bg); border-radius: 5px; padding: 4px 6px; font-size: 13px; color: var(--text); outline: none; }
  .link-row input:focus { border-color: var(--accent); box-shadow: 0 0 0 3px var(--accent-soft); }
  .link-row input[aria-invalid='true'] { border-color: var(--danger); }
  .inline-error { margin: 0 6px 0 28px; font-size: 12px; color: var(--danger); }
  .adds { display: flex; gap: 4px; flex-wrap: wrap; }
  .add { display: inline-flex; align-items: center; gap: 6px; border: none; background: none; padding: 4px 6px; border-radius: 6px; font-size: 13px; color: var(--muted); cursor: pointer; }
  .add:hover { background: var(--hover); color: var(--text); }
  .add:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
  .add:disabled { opacity: 0.6; cursor: default; }
</style>
