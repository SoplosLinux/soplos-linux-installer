<script lang="ts">
  import { onMount } from "svelte";
  import { _ } from "svelte-i18n";
  import { UserRound, KeyRound, ShieldCheck } from "lucide-svelte";
  import { api } from "../lib/api";
  import { plan, canContinue, type InstallPlan } from "../lib/wizard";
  import { loginFrom, hostnameFrom, loginProblem, hostnameProblem, strength } from "../lib/users";
  import PasswordInput from "../components/PasswordInput.svelte";
  import { autofocus } from "../lib/autofocus";

  // DESIGN.md, section 4.5: full name, login, computer name and password,
  // and root with the same password or its own, as Calamares does.

  let edition = "";
  onMount(async () => {
    const env = await api.invoke<{ edition: string }>("get_environment_info").catch(() => ({ edition: "" }));
    edition = env.edition === "unknown" ? "" : env.edition;
  });

  // The login and the computer name follow the full name until the user
  // writes them by hand. Kept by value: if they still match what would be
  // proposed, they were not touched.
  $: u = $plan.user;
  function set(patch: Partial<InstallPlan["user"]>) {
    plan.update((p) => ({ ...p, user: { ...p.user, ...patch } }));
  }

  function onFullName(value: string) {
    const loginTouched = u.login !== "" && u.login !== loginFrom(u.fullName);
    const hostTouched = u.hostname !== "" && u.hostname !== hostnameFrom(u.login, edition);
    const login = loginTouched ? u.login : loginFrom(value);
    set({ fullName: value, login, hostname: hostTouched ? u.hostname : hostnameFrom(login, edition) });
  }

  function onLogin(value: string) {
    const hostTouched = u.hostname !== "" && u.hostname !== hostnameFrom(u.login, edition);
    const login = value.toLowerCase();
    set({ login, hostname: hostTouched ? u.hostname : hostnameFrom(login, edition) });
  }

  // Coming back to the step: the passwords already matched to get past it.
  let confirm = $plan.user.password;
  let rootConfirm = $plan.user.rootPassword;

  $: loginError = u.login ? loginProblem(u.login) : null;
  $: hostError = u.hostname ? hostnameProblem(u.hostname) : null;
  $: mismatch = confirm !== "" && confirm !== u.password;
  $: rootMismatch = rootConfirm !== "" && rootConfirm !== u.rootPassword;
  $: level = strength(u.password);

  $: canContinue.set(
    u.fullName.trim() !== "" &&
      !loginProblem(u.login) &&
      !hostnameProblem(u.hostname) &&
      u.password !== "" &&
      confirm === u.password &&
      (u.sameRootPassword || (u.rootPassword !== "" && rootConfirm === u.rootPassword)),
  );

  const LEVEL_COLOR = ["bg-border", "bg-danger", "bg-warning", "bg-success"];

  const inputClass =
    "w-full bg-surface-2 border rounded px-3 py-1.5 text-[0.88rem] text-text-bright placeholder:text-text-muted outline-none focus-visible:ring-2 focus-visible:ring-primary/60";
</script>

<!-- Every field keeps a fixed line under it for its message, so nothing
     moves while typing. -->
<div class="flex flex-col gap-3">
  <div class="rounded-lg border border-border bg-surface px-5 py-4 flex flex-col gap-1">
    <p class="flex items-center gap-2 text-[0.88rem] text-text-bright font-semibold mb-1.5">
      <UserRound size={18} class="text-primary" />
      {$_("user.account")}
    </p>

    <div class="grid grid-cols-[11rem_minmax(0,1fr)] items-center gap-x-3">
      <label for="full-name" class="text-[0.85rem] text-text">{$_("user.fullName")}</label>
      <input
        id="full-name"
        use:autofocus
        type="text"
        spellcheck="false"
        value={u.fullName}
        on:input={(e) => onFullName(e.currentTarget.value)}
        class="{inputClass} border-border"
      />
      <span></span>
      <span class="h-5"></span>

      <label for="login" class="text-[0.85rem] text-text">{$_("user.login")}</label>
      <input
        id="login"
        type="text"
        spellcheck="false"
        autocapitalize="off"
        value={u.login}
        on:input={(e) => onLogin(e.currentTarget.value)}
        class="{inputClass} {loginError ? 'border-danger' : 'border-border'}"
      />
      <span></span>
      <span class="h-5 text-[0.75rem] pt-0.5 {loginError ? 'text-danger' : 'text-text-muted'}">
        {loginError ? $_(`user.problems.${loginError}`) : $_("user.loginHint")}
      </span>

      <label for="hostname" class="text-[0.85rem] text-text">{$_("user.hostname")}</label>
      <input
        id="hostname"
        type="text"
        spellcheck="false"
        autocapitalize="off"
        value={u.hostname}
        on:input={(e) => set({ hostname: e.currentTarget.value })}
        class="{inputClass} {hostError ? 'border-danger' : 'border-border'}"
      />
      <span></span>
      <span class="h-5 text-[0.75rem] pt-0.5 {hostError ? 'text-danger' : 'text-text-muted'}">
        {hostError ? $_(`user.problems.${hostError}`) : $_("user.hostnameHint")}
      </span>
    </div>
  </div>

  <div class="rounded-lg border border-border bg-surface px-5 py-4 flex flex-col gap-1">
    <p class="flex items-center gap-2 text-[0.88rem] text-text-bright font-semibold mb-1.5">
      <KeyRound size={18} class="text-primary" />
      {$_("user.password")}
    </p>

    <div class="grid grid-cols-[11rem_minmax(0,1fr)_minmax(0,1fr)] items-center gap-x-3">
      <span class="text-[0.85rem] text-text">{$_("user.password")}</span>
      <PasswordInput value={u.password} placeholder={$_("user.password")} onInput={(v) => set({ password: v })} />
      <PasswordInput value={confirm} placeholder={$_("user.repeat")} invalid={mismatch} onInput={(v) => (confirm = v)} />
      <span></span>
      <span class="h-5 flex items-center gap-2 pt-0.5">
        {#each [1, 2, 3] as step}
          <span class="h-1.5 flex-1 rounded-full {level >= step ? LEVEL_COLOR[level] : 'bg-border'}"></span>
        {/each}
        <span class="text-[0.75rem] text-text-muted w-16 text-right">{level ? $_(`user.strength.${level}`) : ""}</span>
      </span>
      <span class="h-5 text-[0.75rem] pt-0.5 text-danger">{mismatch ? $_("user.problems.mismatch") : ""}</span>
    </div>

    <label class="flex items-center gap-2.5 cursor-pointer mt-1">
      <input
        type="checkbox"
        class="w-4 h-4 accent-primary"
        checked={u.sameRootPassword}
        on:change={(e) => {
          set({ sameRootPassword: e.currentTarget.checked, rootPassword: "" });
          rootConfirm = "";
        }}
      />
      <ShieldCheck size={15} class="text-primary" />
      <span class="text-[0.85rem] text-text-bright">{$_("user.sameRoot")}</span>
    </label>

    <!-- The root fields keep their place, disabled, so the card never changes size. -->
    <div class="grid grid-cols-[11rem_minmax(0,1fr)_minmax(0,1fr)] items-center gap-x-3 mt-3 {u.sameRootPassword ? 'opacity-40 pointer-events-none' : ''}">
      <span class="text-[0.85rem] text-text">{$_("user.rootPassword")}</span>
      <PasswordInput value={u.rootPassword} placeholder={$_("user.rootPassword")} onInput={(v) => set({ rootPassword: v })} />
      <PasswordInput value={rootConfirm} placeholder={$_("user.repeat")} invalid={rootMismatch} onInput={(v) => (rootConfirm = v)} />
      <span></span>
      <span></span>
      <span class="h-5 text-[0.75rem] pt-0.5 text-danger">{rootMismatch ? $_("user.problems.mismatch") : ""}</span>
    </div>
  </div>
</div>
