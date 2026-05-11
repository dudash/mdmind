- AI Model Benchmark Comparison #table @snapshot:2026-05-11 [id:model-benchmark]
  | Column-first example: each child row is a model snapshot, and detail lines carry source notes.
  | Suggested Table View columns: provider mode aa_index gpqa hle simplebench swe_verified gdpval best_for caveat source.
  | Sources checked May 11, 2026: Artificial Analysis, LMSpeed, LM Market Cap, and LM Council.
  - GPT-5.5 xhigh #model @provider:openai @mode:xhigh @aa_index:60 @gpqa:na @hle:44.3 @simplebench:na @swe_verified:88.7 @gdpval:na @best_for:frontier-general @caveat:variant-mixed @source:aa-lmc-lmm [id:model-benchmark/gpt-5-5-xhigh]
    | Artificial Analysis lists GPT-5.5 xhigh as the top Intelligence Index result at 60.
    | LM Council lists GPT-5.5 xhigh at 44.3 on Humanity's Last Exam.
    | LM Market Cap lists GPT-5.5 and GPT-5.5 Pro at 88.7 on SWE-bench Verified; the SWE row is not mode-aligned with xhigh.
  - Claude Opus 4.7 max #model @provider:anthropic @mode:max @aa_index:57.3 @gpqa:91.4 @hle:na @simplebench:na @swe_verified:87.6 @gdpval:na @best_for:coding-agents @caveat:mode-sensitive @source:lmspeed-lmm [id:model-benchmark/claude-opus-4-7-max]
    | LMSpeed reports Claude Opus 4.7 at 57.3 on the Artificial Analysis Intelligence Index and 91.4 on GPQA.
    | LM Market Cap lists Claude Opus 4.7 at 87.6 on SWE-bench Verified.
    | LM Council lists Claude Opus 4.7 xhigh at 97.8 on OTIS Mock AIME, which is useful but not included as a primary table column.
  - Gemini 3.1 Pro Preview #model @provider:google @mode:preview @aa_index:57.2 @gpqa:94.1 @hle:44.7 @simplebench:79.6 @swe_verified:80.6 @gdpval:na @best_for:reasoning @caveat:preview @source:lmspeed-lmc-lmm [id:model-benchmark/gemini-3-1-pro-preview]
    | LMSpeed reports Gemini 3.1 Pro at 57.2 on the Artificial Analysis Intelligence Index and 94.1 on GPQA.
    | LM Council lists Gemini 3.1 Pro Preview at 44.7 on HLE and 79.6 on SimpleBench.
    | LM Market Cap lists Gemini 3.1 Pro at 80.6 on SWE-bench Verified.
  - GPT-5.4 xhigh #model @provider:openai @mode:xhigh @aa_index:56.8 @gpqa:92.0 @hle:41.6 @simplebench:na @swe_verified:80.0 @gdpval:83.0 @best_for:knowledge-work @caveat:mode-mixed @source:lmspeed-lmc-lmm [id:model-benchmark/gpt-5-4-xhigh]
    | LMSpeed reports GPT-5.4 at 56.8 on the Artificial Analysis Intelligence Index and 92.0 on GPQA.
    | LM Council lists GPT-5.4 xhigh at 41.6 on HLE and GPT-5.4 at 83.0 on GDPval.
    | LM Market Cap lists GPT-5.4 at 80.0 on SWE-bench Verified.
  - GPT-5.3 Codex xhigh #model @provider:openai @mode:xhigh @aa_index:53.6 @gpqa:91.5 @hle:na @simplebench:na @swe_verified:na @gdpval:70.9 @best_for:coding-agents @caveat:bench-gap @source:lmspeed-lmc [id:model-benchmark/gpt-5-3-codex-xhigh]
    | LMSpeed reports GPT-5.3 Codex at 53.6 on the Artificial Analysis Intelligence Index and 91.5 on GPQA.
    | LM Council lists GPT-5.3 Codex at 349.5 minutes on METR Time Horizons and 70.9 on GDPval.
    | Leave SWE-bench blank here rather than mixing a Codex system score from another harness.
  - DeepSeek V4 Pro #model @provider:deepseek @mode:standard @aa_index:51.5 @gpqa:88.8 @hle:na @simplebench:na @swe_verified:80.6 @gdpval:na @best_for:value-check @caveat:provider-gap @source:lmspeed-lmm [id:model-benchmark/deepseek-v4-pro]
    | LMSpeed reports DeepSeek V4 Pro at 51.5 on the Artificial Analysis Intelligence Index and 88.8 on GPQA.
    | LM Market Cap lists DeepSeek V4 Pro at 80.6 on SWE-bench Verified.
    | Useful as a comparison row when the decision is not only frontier closed-model quality.
