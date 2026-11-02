# AI Search — FRX

> **Piloto Qwen encerrado em 02/11/2026, por decisão do responsável e sem execução.** O modelo não foi baixado, as 50 entradas não foram concluídas e não existe taxa medida. Não há evidência de vantagem da Qwen sobre a abordagem determinística, e também não há comparação direta entre as duas. A decisão se apoia em [D-025](RXAI.md#d-025) (seguir pistas por regra não superou uma pesquisa única) e [D-026](RXAI.md#d-026) (melhorar só a busca não resolveria o gargalo de entender o que foi lido). `ai/qwen.rs` e `ai/query.rs` ficam no código como **experimentais**, fora do caminho crítico. O FRX segue "Deterministic-First".

Módulo de pesquisa, ordenação e sumarização de correções para jogos Windows no Linux,
com contexto de hardware. Faz parte do FRX, mas não substitui o Fix Engine.

## O que é e o que não é

**Faz:**
- Formula queries de busca a partir de um sintoma + `SystemSnapshot` + `GameInfo`
- Consulta providers enumeráveis (allowlist fechada)
- Ranqueia candidatos por pertinência ao hardware/sistema do usuário
- Sumariza com citação literal, sem reescrever o fix
- Entrega candidatos ao Fix Engine via `prepare_fix_plan`

**Não faz:**
- Não diagnostica. Não interpreta procedimentos.
- Não infere causalidade nem associa ação a resultado.
- Não promete FPS.
- Não executa shell, sudo, Winetricks, scripts.
- Não toca em prefixos, drivers, Proton.
- Não baixa nem instala arquivos fora do Fix Engine.

O fundamento está no [RXAI](RXAI.md): D-002 (recuperação sobreviveu em três coortes),
D-008 (modalidade ainda confunde execução, sugestão e resultado), D-010 (11/70
candidatos completos limitaram o classificador no v6). A conclusão é que o
recuperador é promissor e o intérprete não é — o módulo de busca respeita essa
fronteira.

## Reescrita de consulta

`ai/query.rs` transforma sintoma, hardware e jogo em uma consulta por provedor. A implementação determinística é o default. `ai/qwen.rs`, com a feature `qwen-query`, é opcional e passa pelo mesmo validador de origem. Um termo só entra se veio do input, da tabela de palavras-chave aplicada ao sintoma ou de uma palavra fixa de template (`umu`, `linux`).

O default é a implementação determinística. O piloto em `src-tauri/tests/fixtures/query_pilot.jsonl` foi encerrado com 47 entradas, e um teste mantém esse tamanho congelado. Com menos de 50, o relatório continua dizendo que não há decisão. As instruções abaixo ficam só como registro, caso o experimento seja retomado.

Para preencher uma linha do piloto, substitua um `TODO` por uma entrada completa com `gameName`, `system` e `expected`, e use `"labeledBy": "human"`. Só essas linhas entram na métrica; `"example"` fica de fora. `expected` precisa ser rotulado por uma pessoa: rótulo gerado por modelo não vale como medida.

```fish
cd src-tauri
cargo test --package frx ai_query_pilot_eval -- --ignored --nocapture
cargo test --package frx --features qwen-query ai_query_pilot_eval -- --ignored --nocapture
```

O relatório sai em `src-tauri/target/query-pilot-report.md`. O modelo fica em `~/.local/share/io.frx.FRX/ai/qwen2.5-3b-instruct-q4_k_m.gguf`, baixado à mão. A licença dele é `qwen-research`, que restringe uso comercial.

## Limitações conhecidas

- driver_proximity retorna null quando os dois lados são de vendors diferentes ou quando a versão do Mesa não pôde ser lida com confiança. null não penaliza. É tratado como 'sem informação', não como 'sem match'.
- A versão do Mesa vem só de leitura de arquivos: banco local do pacman, `/var/lib/dpkg/status` ou o nome de `libgallium-<versão>.so`. Fontes que discordam dão null. Em sistemas RPM, só o nome da biblioteca serve, e ele existe a partir do Mesa 24.2.
- A leitura diz o que está instalado no sistema. Um Steam em Flatpak usa o próprio Mesa, que o FRX não enxerga.
- Os pesos do score são provisórios.
- A tabela de palavras-chave não cobre fps, XeSS, frame generation, DLL ou prefixo: nesses casos, a consulta usa as palavras literais do sintoma. Sintoma em outro idioma não é traduzido.
- Com a Qwen, `research_fix` roda a inferência na thread do comando e pode travar a janela. É aceitável só para teste manual.

## Arquitetura
