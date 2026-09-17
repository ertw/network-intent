<script lang="ts">
  const title = 'Second development fixture — glob discovery check';
  const body =
    'harness-discovery exists only to prove a second Demo.svelte is found without editing App.svelte, package files, or a name registry. Not live data.';
</script>

<section class="fixture" aria-labelledby="harness-discovery-title">
  <p class="label">DEVELOPMENT FIXTURE — NOT LIVE DATA</p>
  <h2 id="harness-discovery-title">{title}</h2>
  <p>{body}</p>
</section>

<style>
  .fixture {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .label {
    font-family: var(--mono);
    font-size: 0.78rem;
    letter-spacing: 0.06em;
    font-weight: 700;
    color: var(--danger);
  }

  h2 {
    font-size: 1.15rem;
    line-height: 1.3;
  }
</style>
