<script lang="ts">
  const title = 'Development fixture only — not live data';
  const body =
    'harness-example is a test-only loader target. It is not device configuration, operational state, or an admission decision.';
</script>

<section class="fixture" aria-labelledby="harness-example-title">
  <p class="label">DEVELOPMENT FIXTURE — NOT LIVE DATA</p>
  <h2 id="harness-example-title">{title}</h2>
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
