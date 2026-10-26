# Fix Engine dinâmico (gerador de receitas)

O FRX gera receitas sozinho a partir das descobertas da pesquisa, sem cadastro manual, quando reconhece e valida uma ação suportada. O catálogo revisado (`src-tauri/recipes/`) continua existindo, mas deixou de ser o único caminho para aplicar uma correção.

Código: `src-tauri/src/fix_engine/dynamic/`.

## O bloqueio que existia (v0.4.0)

1. `FixEngine::prepare/apply` só procuravam a receita em `Catalog::bundled()`, embutido no binário com `include_str!`. Adicionar uma receita exigia editar JSON, atualizar o SHA no manifesto, recompilar e lançar uma versão nova.
2. O planejador bloqueava tudo o que não fosse `Trust::Reviewed`.
3. Não havia conversão de `FixFinding` (descoberta) em `Recipe`. O cartão só mostrava um botão quando uma receita estática tinha `relatedKeys` igual ao ajuste da descoberta. Por isso `PROTON_NO_D3D11=1`, encontrada no Proton#309, aparecia como “Receita candidata · só consulta”.

O AppID estava correto (a descoberta já carrega `gameAppId`). O que faltava era o caminho descoberta → receita.

## Fluxo

```
FixReport (cache da pesquisa, lido pelo núcleo)
  → provenance  só fontes HTTPS do github.com, de provedores integrados
  → extraction  interpretador determinístico: cada FixAction é reinterpretada com parsers estritos
  → validation  segurança da operação (NOME=valor) e confiabilidade (evidência), separadas
  → analyzer    categoria A/B/C + motivo específico; deduplicação por ID estável
  → generator   Recipe { trust: "generated", generated: { origem… } }
  → registry    fix-engine/generated/<appid>.json (0700/0600)
  → planner     mesma prévia, backup, diário, verificação e rollback do catálogo
```

Cada descoberta é analisada sozinha. Ações de comentários diferentes nunca são somadas numa correção.

## Categorias

| Categoria | Estados (`DynStatus`) | Botão |
|---|---|---|
| **A — aplicável com confirmação** | `generated` (receita gerada), `reviewed` (uma receita revisada do catálogo faz exatamente o mesmo ajuste) | **Aplicar correção** → abre a prévia, nunca aplica direto |
| **B — requer validação adicional** | `needs-validation` (winetricks, DLL, registro, arquivo do jogo, wrappers como `gamemoderun`, argumentos do jogo, variável desconhecida, script condicional ou parcialmente analisado), `blocked` (variáveis que carregam código ou mudam caminhos: `LD_*`, `WINEDLLOVERRIDES`, `PATH`, camadas Vulkan, `STEAM_*`…) | desabilitado, com o motivo |
| **C — apenas informativa** | `no-actions`, `contested` (relatos contrários ≥ favoráveis), `incompatible`, `already-included`, `invalid` (AppID diferente, conteúdo malformado, sem fonte rastreável) | desabilitado, com o motivo |

Uma correção com partes não suportadas nunca é aplicada parcialmente. Por exemplo, `gamemoderun PROTON_NO_ESYNC=1 %command%` vira categoria B inteira.

## Segurança × confiabilidade

- **Segurança** (`validation::env`): o par `NOME=valor` precisa passar pela `security::ENV_ALLOWLIST` (formato do valor) **e** estar em `compatibility::KNOWLEDGE` (efeito, risco e GPU exigida conhecidos). Variáveis de diagnóstico (`PROTON_LOG`, `DXVK_HUD`) não viram correção.
- **Confiabilidade** (`validation::evidence`):
  - `documented`: UMU-Protonfixes ou mantenedor;
  - `corroborated`: 2 ou mais relatos favoráveis, mais que os contrários;
  - `single`: um relato, marcado **experimental**;
  - `none`: sem relatos, também experimental;
  - `contested`: categoria C.

A evidência é exibida, mas não libera a aplicação. O que libera é a segurança da operação somada a evidência não contestada.

### Variáveis que podem gerar receita

`PROTON_NO_ESYNC=1`, `PROTON_NO_FSYNC=1`, `PROTON_NO_NTSYNC=1`, `PROTON_USE_WINED3D=1`, `PROTON_NO_D3D11=1`, `PROTON_NO_D3D10=1`, `PROTON_FORCE_LARGE_ADDRESS_AWARE=1`, `PROTON_HEAP_DELAY_FREE=1`, `PROTON_OLD_GL_STRING=1`, `PROTON_USE_XALIA=0`, `DXVK_FRAME_RATE=10…360` e `VKD3D_FRAME_RATE=10…360`. Exigem NVIDIA: `PROTON_DISABLE_NVAPI=1`, `PROTON_ENABLE_NVAPI=1` e `PROTON_HIDE_NVIDIA_GPU=1`. As quatro adicionadas nesta versão (`HEAP_DELAY_FREE`, `OLD_GL_STRING`, `USE_XALIA`, `DISABLE_NVAPI`) foram conferidas na tabela “Runtime Config Options” do README do Proton 10.

## Validações da receita gerada

`security::validate_generated` revalida cada campo, de forma independente do gerador:

- `trust = generated`, sem `review`, com registro de origem;
- ID `gen.<appid>.<16 hex>`;
- um único AppID no escopo, que exige Proton;
- de 1 a 4 ações, só `set_launch_env`, cada uma na allowlist **e** na base de conhecimento;
- sem `conflictsWith`;
- textos sem caracteres de controle;
- fontes HTTPS do GitHub.

O catálogo recusa receitas com `generated` ou `trust = generated`. O planejador chama `validate_generated` de novo, e a prévia mostra o aviso “Receita gerada”.

## ID estável e deduplicação

`gen.<appid>.<sha256(appid + NOME=valor ordenados)[..16]>`. O ID não depende do texto exibido nem da ordem das variáveis. Descobertas diferentes com as mesmas ações viram uma receita só, com todas as origens (`generated.findingIds` e `provenance`) e a melhor evidência entre elas.

## Execução

- A interface envia só `recipeId` + `appId` (prévia) e `planId` + `fingerprint` + `appId` (aplicação).
- O núcleo **regenera** a receita a partir do cache da pesquisa (`analyzer::resolve`) antes da prévia e de novo antes da aplicação. O registro em disco nunca é executado.
- A impressão digital da prévia inclui o SHA-256 da receita. Se a descoberta mudar entre a prévia e a confirmação (sumiu, ganhou relatos contrários, mudou a evidência), a aplicação é recusada sem gravar nada.
- O restante é o Fix Engine da v0.4.0, sem mudanças:
  - backup verificado e gravação atômica;
  - verificação e diário;
  - modo manual com a Steam aberta;
  - conflito com o jogo aberto;
  - rollback seletivo e recuperação.
- As operações registram `recipeTrust` e `findingIds`, e o histórico mostra “receita gerada automaticamente”.

## Registro local e reavaliação

`fix-engine/generated/<appid>.json` guarda, por descoberta:

- a análise completa: receita, categoria, motivo e proveniência com URL e trecho original;
- as entradas: hash da descoberta, Proton, GPU, driver, versão do gerador, versão das regras e SHA do catálogo;
- `schemaVersion`, `discoveredAt`, `generatedAt`, `updatedAt`;
- `reevaluated`, com o motivo, por exemplo: “Reavaliada porque mudou o Proton.”

Se todas as entradas forem iguais, a análise é reaproveitada. Qualquer mudança força uma nova análise. Ao mudar `ENV_ALLOWLIST` ou `KNOWLEDGE`, aumente `RULES_VERSION`. Um registro corrompido é ignorado e refeito.

Nenhum commit, JSON manual ou versão nova é necessário para uma correção que use ações já suportadas.

## Interpretação inteligente (futuro)

`extraction::Interpreter` é o ponto de extensão. Um interpretador baseado em IA teria de devolver o mesmo `Extracted` (`Env { key, value }` ou `Unsupported`) e passaria exatamente pelas mesmas validações. O teste `hostile_interpreter_output_goes_through_the_same_validation` cobre isso. Não existe interpretador de IA nesta versão, nem chave de API. A interpretação de texto não prova que uma correção funciona.

## Catálogo remoto (futuro)

O registro já usa o ID estável e o mesmo formato de receita, o que permite sincronizar com um catálogo remoto assinado. Nenhum servidor é usado nem exigido.

## Comandos

- `analyze_fix_findings(appId)` devolve `AssessmentView[]` (análise + datas do registro). O núcleo lê as descobertas do próprio cache.
- `check_fix_compatibility`, `prepare_fix_plan` e `apply_fix_plan` aceitam IDs `gen.*`.

## Limites

- Só variáveis de ambiente nas opções de inicialização são geradas. Winetricks, DLLs, registro, arquivos do jogo, wrappers e argumentos do jogo continuam na categoria B.
- A extração é determinística sobre as ações que a pesquisa já estrutura. Relatos em prosa sem `NOME=valor` ou `%command%` não geram nada.
- Gerar uma receita não prova que ela funciona: use o Performance Lab para comparar antes e depois.
