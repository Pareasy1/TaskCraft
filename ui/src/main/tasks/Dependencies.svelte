<script lang="ts">
  import { onDestroy, tick } from 'svelte';
  import { Plus, X } from 'lucide-svelte';
  import { t } from '../../lib/i18n';
  import { api, errorMessage, onItemsChanged } from '../../lib/api';
  import { blocking, pickerExclude } from '../../lib/dependencies';
  import { notify } from '../../lib/toast.svelte';
  import type { Dependency, Task } from '../../lib/types';
  import StatusGlyph from './StatusGlyph.svelte';
  import TaskPicker from './TaskPicker.svelte';

  let { task, onopentask }: { task: Task; onopentask: (id: number) => void } = $props();

  let deps = $state<Dependency[]>([]);
  let all = $state<Task[]>([]);
  let picking = $state(false);
  let error = $state('');
  let addBtn = $state<HTMLButtonElement | undefined>();
  let listEl = $state<HTMLElement | undefined>();

  async function load() {
    const id = task.id;
    try {
      const [d, a] = await Promise.all([api.listDependencies(), api.listTasks('all')]);
      if (task.id !== id) return;
      deps = d;
      all = a;
    } catch (e) {
      error = errorMessage(e);
    }
  }
  $effect(() => {
    void task.id;
    picking = false;
    error = '';
    load();
  });
  const unlisten = onItemsChanged(() => load());
  onDestroy(() => void unlisten.then((f) => f()));

  const byId = $derived(new Map(all.map((x) => [x.id, x])));
  const dependsOn = $derived(
    deps.filter((d) => d.taskId === task.id).map((d) => byId.get(d.dependsOn)).filter((x): x is Task => !!x),
  );
  const blocks = $derived(blocking(task.id, deps, all));
  // `deps` is the whole graph, so loops through other tasks are caught too.
  const excluded = $derived(pickerExclude(task.id, deps));

  async function link(other: Task, add: boolean): Promise<void> {
    if (add) await api.addDependency(task.id, other.id);
    else await api.removeDependency(task.id, other.id);
    await load();
  }
  async function change(other: Task, add: boolean) {
    error = '';
    try {
      await link(other, add);
    } catch (e) {
      error = errorMessage(e);
      return false;
    }
    notify(t(add ? 'dep.added' : 'dep.removed', { title: other.title }), () => link(other, !add));
    return true;
  }
  async function pick(other: Task) {
    picking = false;
    await change(other, true);
    await tick();
    addBtn?.focus();
  }
  async function remove(other: Task) {
    const rows = dependsOn.map((x) => x.id);
    const next = rows[rows.indexOf(other.id) + 1] ?? rows[rows.indexOf(other.id) - 1];
    if (!(await change(other, false))) return;
    await tick();
    ((next == null ? null : listEl?.querySelector<HTMLElement>(`[data-dep-remove="${next}"]`)) ?? addBtn)?.focus();
  }
  function cancel() {
    picking = false;
    tick().then(() => addBtn?.focus());
  }
</script>

<section class="deps" aria-label={t('dep.section')} bind:this={listEl}>
  <h3>{t('dep.section')}</h3>
  <h4>{t('dep.dependsOn')}</h4>
  {#if dependsOn.length}
    <ul>
      {#each dependsOn as x (x.id)}
        <li>
          <StatusGlyph status={x.status} size={14} />
          <button class="name" onclick={() => onopentask(x.id)} title={t('dep.openNamed', { title: x.title })}>{x.title}</button>
          <button class="icon-btn act" data-dep-remove={x.id} aria-label={t('dep.removeNamed', { title: x.title })} title={t('dep.removeNamed', { title: x.title })}
            onclick={() => remove(x)}><X size={13} aria-hidden="true" /></button>
        </li>
      {/each}
    </ul>
  {:else}
    <p class="none">{t('dep.none')}</p>
  {/if}
  {#if picking}
    <TaskPicker tasks={all} exclude={excluded} onpick={pick} oncancel={cancel} />
  {:else}
    <div class="adds">
      <button class="add" bind:this={addBtn} onclick={() => (picking = true)}><Plus size={13} aria-hidden="true" />{t('dep.add')}</button>
    </div>
  {/if}
  {#if error}<p class="inline-error" role="alert">{error}</p>{/if}
  {#if blocks.length}
    <h4>{t('dep.blocks')}</h4>
    <ul>
      {#each blocks as x (x.id)}
        <li>
          <StatusGlyph status={x.status} size={14} />
          <button class="name" onclick={() => onopentask(x.id)} title={t('dep.openNamed', { title: x.title })}>{x.title}</button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .deps { display: flex; flex-direction: column; gap: 4px; }
  h3 { margin: 4px 0 2px; font-size: 12px; font-weight: 600; color: var(--muted); }
  h4 { margin: 2px 6px 0; font-size: 11.5px; font-weight: 500; color: var(--faint); }
  ul { list-style: none; margin: 0; padding: 0; display: flex; flex-direction: column; }
  li { display: flex; align-items: center; gap: 8px; padding: 2px 6px; border-radius: 7px; min-height: 30px; }
  li:hover { background: var(--hover); }
  .name { flex: 1; min-width: 0; text-align: left; border: none; background: none; padding: 2px 0; font-size: 13px; color: var(--text); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; cursor: pointer; border-radius: 4px; }
  .name:hover { text-decoration: underline; }
  .name:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
  .none { margin: 0 6px; font-size: 12.5px; color: var(--muted); }
  .act { padding: 2px; opacity: 0; flex: none; }
  li:hover .act, li:focus-within .act, .act:focus-visible { opacity: 1; }
  .inline-error { margin: 0 6px; font-size: 12px; color: var(--danger); }
  .adds { display: flex; }
  .add { display: inline-flex; align-items: center; gap: 6px; border: none; background: none; padding: 4px 6px; border-radius: 6px; font-size: 13px; color: var(--muted); cursor: pointer; }
  .add:hover { background: var(--hover); color: var(--text); }
  .add:focus-visible { outline: 2px solid var(--accent); outline-offset: -2px; }
</style>
