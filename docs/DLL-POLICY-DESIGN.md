# Política de DLL e TAR: design

**Estado:** proposta simplificada. Nada aqui instala, baixa, extrai ou verifica binários. O código tem só os tipos: `ArtifactFormat::Tar`, sempre recusado, `DllArtifact`, que nenhum fluxo usa, e `Operation.requires_explicit_user_consent`, sempre `false` enquanto não houver esse fluxo.

**Prioridade do FRX:** variáveis de inicialização da Steam. DLL é uma exceção, para casos como um arquivo publicado que vai na pasta do jogo.

## Escopo de escrita

Só a pasta do jogo, em `steamapps/common/<jogo>`. O `confined.rs` não será expandido para o prefixo.

| Tentativa | Resultado |
|---|---|
| Raiz em `compatdata/` ou `pfx/` | Recusada por `check_game_root`, com mensagem que cita `compatdata/` |
| Caminho relativo com um componente `compatdata` | Recusado por `Target::open` |
| `..` no caminho | Recusado por `check_rel_path` |
| Symlink na pasta do jogo apontando para o prefixo | Recusado por `O_NOFOLLOW` |

Esses quatro casos têm testes em `confined.rs`.

**Consequência:** uma correção que dependa de prefixo, registro do Wine, override de DLL ou alteração da instalação do Proton continua em revisão. O `nvidia-libs` documenta esses procedimentos, então este design não o torna instalável.

## Segurança

- **`expectedSha256` é obrigatório** e autoriza a instalação. O hash vem de uma receita revisada, nunca de um comentário.
- **O hash prova só que o arquivo é o da receita.** Ele não prova que o arquivo é benigno, compatível ou mais rápido. Sem Authenticode, a revisão humana da receita é a única avaliação do conteúdo.
- **Receitas geradas a partir de descobertas nunca instalam DLL.** Elas continuam em revisão, e fonte da comunidade só serve como pista.
- **`requires_explicit_user_consent` será `true`** em toda operação com binário. O campo só registra o consentimento: `prepare_fix_plan` declara a exigência, e `apply_fix_plan` recusa no backend sem a confirmação ligada ao `fingerprint` do plano. Uma trava só na interface não protege.
- **A tela de confirmação mostra** origem, hash, destino, o que será substituído e a frase "o FRX não testou este binário", com a caixa de risco desmarcada.

## Tipos

| Campo de `DllArtifact` | Regra |
|---|---|
| `sourcePath` | Membro exato do pacote, ou `null` se o download for a própria DLL |
| `targetPath` | Relativo à raiz do jogo, validado na escrita |
| `expectedSha256` | Obrigatório, sem valor default |
| `expectedOriginalSha256` | Hash da DLL substituída. `null` só quando o destino não existe |

O tipo usa `deny_unknown_fields`: um manifesto com campo desconhecido falha.

## TAR

`ArtifactFormat::Tar` serve para extrair arquivos específicos para a pasta do jogo, com as mesmas restrições do ZIP: membros selecionados um a um, staging privado e SHA-256 por membro. Hoje `validate` e `contents` recusam esse formato.

Na implementação:

- **Tipos de entrada:** só arquivo comum e diretório. Link simbólico, hard link, dispositivo e FIFO recusam o pacote inteiro.
- **Nomes:** aplicar `safe_member` depois de resolver os nomes longos GNU e PAX. Nome duplicado, com qualquer caixa, recusa o pacote.
- **Compressão:** o `.tar.xz` exige dependências novas, para TAR e XZ. Limitar os bytes expandidos durante a leitura.
- **Extração:** nunca extrair o pacote inteiro.

## Rollback

Reaproveita a troca com `RENAME_EXCHANGE` de `confined.rs`, sem `flock`.

**Aplicar:**

1. Verificar o artefato por completo.
2. Conferir que o jogo está fechado, com `game_running`.
3. Conferir `expectedOriginalSha256`. Diferente, para sem escrever.
4. Guardar o backup fora da pasta do jogo e conferir o hash.
5. Trocar de forma atômica, reler o destino e registrar o hash final.

**Reverter:**

- **Jogo aberto:** recusar, sem aguardar, e manter a operação como `Applied`. O FRX não encerra processos.
- **Destino igual ao hash final:** restaurar o backup e conferir. Se não existia original, remover o arquivo.
- **Destino diferente, ou backup ausente ou corrompido:** não sobrescrever nem apagar. A operação vai para `RecoveryRequired`.

**O que precisa mudar na implementação:** `read`, `write` e `backup` têm o limite de 256 KiB de `MAX_CONFIG`, e `Step::GameFile` guarda conteúdo como `String`. DLL precisa de um passo binário, um limite próprio e um novo `JOURNAL_SCHEMA`.

**Limite aceito:** `game_running` procura `SteamAppId` em `/proc` e devolve `false` se não conseguir ler. Um processo que sobreviva sem essa variável pode manter a DLL antiga mapeada. A troca funciona no Linux, e esse processo continua com a versão anterior até sair.

## Removido desta versão

| Item | Motivo | Quando reabrir |
|---|---|---|
| Authenticode e `publisherAllowlist` | Não há crate estável que verifique por completo, e o escopo ficou na pasta do jogo | Se o FRX quiser aceitar DLLs sem revisão por hash |
| Escrita no prefixo | Afeta todo o ambiente do jogo, e `dosdevices/z:` aponta para `/` | Se um caso real exigir, com novo design |
| `flock` e detecção de `wineserver` | Só eram necessários para o prefixo | Junto com a escrita no prefixo |

## Testes previstos para a implementação

Todos com bytes sintéticos, sem DLL real:

- TAR com traversal, link, hard link, nome PAX malicioso, duplicata e bomba de expansão.
- Hash errado, destino alterado, backup corrompido e falha em cada etapa da escrita.
- `apply_fix_plan` sem consentimento e com consentimento de outro fingerprint.

## Decisões para aprovação

| # | Decisão | Recomendação |
|---|---|---|
| 1 | Dependências de TAR e XZ | Revisar licença e superfície de ataque antes de adicionar |
| 2 | Redistribuição | O FRX nunca espelha binários: baixa da origem e confere a licença na revisão da receita |
