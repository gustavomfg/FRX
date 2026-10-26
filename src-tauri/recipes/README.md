# Catálogo de receitas do FRX

Receitas **revisadas** que o Fix Engine pode aplicar. São embutidas no binário. O formato está em `docs/FIX-ENGINE.md`.

Este catálogo **não** é o único caminho: correções que só definem variáveis conhecidas são geradas automaticamente a partir das descobertas (`docs/FIX-ENGINE-DINAMICO.md`), sem JSON manual nem versão nova. Use o catálogo para receitas revisadas por pessoas, ações que o gerador não produz (argumentos, arquivos do jogo) e diagnóstico.

## Adicionar ou alterar uma receita

1. Crie `frx.<área>.<nome>.json` com `"trust": "candidate"` e sem `review`. Use apenas ações do conjunto fechado e variáveis da allowlist (`src/fix_engine/security.rs`). Fontes precisam ser HTTPS.
2. Revise: confira a fonte, o efeito, os efeitos colaterais e o rollback. Só então mude para `"reviewed"` e preencha `review`.
3. Aumente `version` a cada mudança de comportamento. Operações antigas guardam a versão aplicada.
4. Registre o arquivo em `FILES` (`src/fix_engine/catalog.rs`) e atualize o hash no `catalog.json`:

   ```sh
   sha256sum src-tauri/recipes/<arquivo>.json
   ```

5. Rode `cargo test fix_engine`. O teste `bundled_catalog_is_intact` falha se algum hash, campo ou ação não conferir.

Para retirar uma receita sem apagar o histórico, adicione o ID em `revoked`.

**Nunca** transforme um script do UMU-Protonfixes, uma issue do GitHub ou um comentário de fórum em receita *revisada* sem revisão humana. O gerador dinâmico produz receitas com estado próprio (`generated`), nunca `reviewed`.
