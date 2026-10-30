# FRX AI v6 — resultado de 30/10/2026

**O v6 entrega um protótipo menor e uma avaliação contextual reproduzível, mas
não demonstrou compreensão autônoma confiável.** A cobertura cresceu com mais
erros; não há autorização para recomendações automáticas. Nenhuma mudança foi
feita no aplicativo, Steam, jogos, prefixos ou empacotamento. Sem commits/push.

## Hipóteses e experimentos

Investigamos identidade local de eventos, verificação de grupos completos,
decomposição de afirmações, memória exata por ambiente e replay de investigação.
[Literatura](LITERATURE.md) registra EPAL, JLF, ERE, DrDEE, coreferência, verificação
de afirmações e benchmarks de incidentes, incluindo indisponibilidade de código.

Foram **seis configurações treinadas**, mais **dois refits de reprodução**:
três regressões logísticas multiclasse (atributos estruturais; dados contrastivos;
atributos mais embeddings v2) e três cross-encoders MiniLM (texto real; contraste;
contraste com atributos). Houve também controles determinísticos, proximidade,
NLI pré-treinado, sintaxe Stanza/spaCy, isolamento e replay. Pontos de limiar,
épocas e testes não foram contados como modelos diferentes. `N4` não foi rodado.

A melhor seleção no desenvolvimento foi o classificador estrutural treinado
de **2.141 bytes**, 19 atributos e limiar 0,3. Seus pesos são dados, sem pickle
ou PyTorch na inferência. Identidade e restrições são regras explícitas, não
coreferência aprendida. Nenhum modelo diagnóstico foi treinado.

| Verificador no desenvolvimento novo | Relações corretas/propostas | Falsos sucessos |
|---|---:|---:|
| Estrutural determinístico final | 11/13 | 0 |
| L1 estrutural treinado, selecionado | 11/11 | 0 |
| L2 com contraste sintético | 11/12 | 1 |
| L3 com embeddings congelados | 8/8 | 0 |
| N1 cross-encoder, texto real | 9/15 | 3 |
| N2 cross-encoder, contraste | 10/15 | 2 |
| N3 cross-encoder, contraste + estrutura | 11/15 | 1 |
| NLI existente | 1/1 | 0 |

Havia 21 relações anotadas no desenvolvimento. O NLI só recuperou 4,8%; não foi
aceito como solução por abstenção. Os neurais também não justificaram seu custo.
L2 manteve uma atribuição incorreta de sucesso. Histórico original e
decodificação estão no cache; o snapshot apresenta a última comparação anterior
ao congelamento.

## Dados e protocolo

Treino: 83 fontes antigas de treino/auditoria, sem o teste reservado v5. Geraram
133 afirmações sobre texto real, incluindo alternativas e polaridades construídas,
e 144 afirmações sobre 36 textos explicitamente sintéticos. As classes não são
133 rótulos humanos independentes. Desenvolvimento novo: 48 trechos/seis jogos.
Aceitação: 72 trechos de outros seis jogos, mais dois casos de nome exato e 22
casos enriquecidos por citações/contexto. Todas as fontes são comentários
públicos Proton, até os primeiros 100 por discussão; não representam amostragem
aleatória da comunidade.

Treino e novas discussões não compartilham AppIDs de origem nem duplicatas
canônicas exatas. Isso não prova ausência de todo vazamento semântico. Citações
repetem experiências dentro da aceitação; não contam como confirmações
independentes. Há somente seis clusters de jogo/discussão no teste.

Código/modelos/limiares foram congelados antes de abrir os corpos de aceitação;
os 96 rótulos foram travados antes das previsões. A primeira execução abortou por
um intervalo de citação invertido. Uma correção mínima de limites foi registrada
após o congelamento, sem alterar identidade, pesos, limiares ou rótulos. Nenhuma
métrica havia sido produzida/inspecionada. O congelamento original foi mantido.

Todos os rótulos são **provisórios do agente, sem revisão humana independente**.
Três identidades de ator do desenvolvimento receberam errata versionada, porque
sujeito e execução eram confundidos na anotação. Os rótulos finais permanecem
inalterados, inclusive onde a auditoria revelou problemas de canonicalização.

## Comparação direta no teste novo

| Medida — 72 trechos principais | v5 Stanza | v6 selecionado |
|---|---:|---:|
| Execuções corretas / 43 anotadas | 12 | 19 |
| Execuções propostas | 12 | 23 |
| Precisão / recuperação de execução | 100% / 27,9% | 82,6% / 44,2% |
| Falsas execuções na métrica estrita | 0 | 4 |
| Relações corretas / propostas | 4/4 | 7/9 |
| Relações verdadeiras anotadas | 48 | 48 |
| Recuperação de relações | 8,3% | 14,6% |
| Atribuições incorretas de sucesso | 0 | 2 |
| Relações negativas corretas | 3 | 5 |

Nos **24 desafios adicionais**, v6 teve 9/15 execuções corretas, seis falsas,
2/3 relações corretas e uma atribuição incorreta de sucesso. O v5 teve 4/6
execuções corretas e nenhuma relação. Não misturamos esses casos enriquecidos
com o conjunto principal para estimar prevalência.

Agregado descritivo dos **96 trechos**: v6 **28/38 execuções corretas** (73,7% de
precisão; 37,8% das 74 anotadas), **9/12 relações corretas**, três falsos sucessos
e **61/70 relações não recuperadas**. O v5 teve 16/18 execuções e 4/4 relações,
sem falso sucesso. A maior cobertura não satisfez as prioridades de segurança.

No principal, Wilson descritivo 95%: precisão de execução 62,9–93,0%; relações
45,3–93,7%. Não são garantias populacionais com textos dependentes. Bootstrap
pareado por seis jogos deu intervalo de ganho de recall de relações **0–12,3
pontos percentuais**, incluindo zero. Não há ganho estatístico robusto declarado.

### Contexto, procedimentos e falhas

- O v5 público já deixava jogo/ator desconhecidos. Não atribuímos a ele o controle
  ruim de preencher AppID/autor pela discussão. Esse controle separado fez 77
  atribuições de jogo sem sustentação; o v6 fez uma.
- No agregado, v6 acertou **4/5 atribuições de AppID**, com 11 identidades
  anotadas, e **34/39 contas de ator**, com 55 anotadas. Houve zero trocas de
  AppID conhecido, mas os casos reais quase não contêm pares explícitos de jogos
  diferentes: **não demonstram que coreferência entre jogos está resolvida**.
  O teste sintético de dois jogos funciona; seu estatuto é apenas contratual.
- Nenhuma das **sete relações conjuntas** finais foi recuperada. Um procedimento
  conjunto é reconstruído no desenvolvimento conhecido, não generalizou ao teste.
  Temporalidade também não recebeu anotação exaustiva; nenhuma competência foi
  validada nessa dimensão.
- Citações preservam o ator como citado, sem inventar a conta original. Porém,
  identidade de ator no grupo completo ainda erra; edição de título oficial
  também impede reconhecer alguns nomes antigos, como Sea of Thieves.
- As dez falsas execuções estritas incluem software tratado como humano, mas
  também cabeças verbais diferentes (`Tried`/`fiddling`, `try`/`running`), eventos
  duplicados e omissões de anotação. Não reclamamos esses acertos retroativamente.
  A fila de 52 unidades permite revisão sem alterar a avaliação publicada.

Erros reais remanescentes: `no versions ... get it working` foi interpretado como
sucesso ([fonte](https://github.com/ValveSoftware/Proton/issues/7045#issuecomment-1694514368));
uma alteração de rede recebeu crédito individual por procedimento conjunto
([fonte](https://github.com/ValveSoftware/Proton/issues/3931#issuecomment-638881015));
o jogo `forces you` foi tratado como operação humana
([fonte](https://github.com/ValveSoftware/Proton/issues/1029#issuecomment-458375051)).
Nenhuma dessas instruções foi executada ou aprovada pelo laboratório.

Ganhos pontuais: associar o clique em confirmação ao erro ao salvar
([fonte](https://github.com/ValveSoftware/Proton/issues/1029#issuecomment-458452948))
e recuperar o relato de correção do controle usando Gamescope
([fonte](https://github.com/ValveSoftware/Proton/issues/7045#issuecomment-1699877456)).
O erro ao salvar aparece também na mensagem original citada: dois trechos não
são duas experiências independentes. Os exemplos executáveis exibem tanto
esses avanços quanto falhas.

## Descoberta principal e diagnóstico

O gargalo está **antes do verificador**: só **11/70** relações verdadeiras tinham
ação, resultado e grupo completo entre os candidatos. Mesmo um classificador
perfeito não superaria 15,7% de recuperação nesse gerador. Das 61 relações
perdidas, 14 pertencem a estados preexistentes/condicionais que o gerador exclui;
47 já têm proprietários executados, mas sofrem perdas de spans, observações ou
agrupamento. Trocar apenas o classificador não resolve essa representação.

A curva risco–cobertura pós-hoc permanece descritiva: limiar >=0,5 rejeita todas
as relações. Nenhum limiar foi alterado depois do teste. Escolher outro usando
esses resultados exigiria uma nova aceitação independente; nove acertos em
nove previsões não validariam precisão geral de 100%.

O replay conhecido usa snapshots e revelação de logs, sem respostas esperadas
no input: **10/10 próximas verificações** no v6 contra 8/10 no v5. É controle de
engenharia com templates desenvolvidos dessas fontes, não precisão diagnóstica
independente. Três probes pós-congelamento de fontes novas tiveram 3/3 em ambos.
Não houve causa confirmada ou probabilidade causal inventada. Logs de símbolos
e áudio motivam perguntas sobre componente/versão/dispositivo, não instalações.

A memória persistente impede repetir uma ação explicitamente malsucedida no
mesmo ambiente exato e distingue driver/hashes/jogo/sintoma. Fontes de outro
hardware continuam como analogias incompatíveis; não viram resultados locais.
Isso é lógica determinística testada, não aprendizado de causas. O recuperador
neural v2 permanece intacto e ordena evidências; **nenhum novo ganho de NDCG ou
competência diagnóstica aprendida foi demonstrado**.

## Reprodução e custo

Seeds após parsing/tokenização; hashes de matrizes, tensores, fontes, pesos,
divisões e dependências registrados. N2/N3 usaram tensores idênticos; suas
diferenças não vêm de cache/RNG do parser. Dois refits lineares com seed de
controle deram diferença máxima de coeficientes **zero**. Uma nova reconstrução
sem cache produziu métricas **idênticas dos seis comparadores**, por conjunto e
por jogo. Não é um novo teste independente.

CPU Ryzen 5 4600G, quatro threads; RTX4060 detectada, não utilizada. Nenhum peso
novo baixado. Ajustes neurais: 317 segundos no total; estudos lineares: cerca de
oito segundos, mais controles curtos. Pico de RAM de treinamento não foi medido
separadamente. Cache novo aproximadamente 65 MiB, abaixo do orçamento de 2 GiB;
dependências e modelos existentes foram reaproveitados.

Benchmark em processos novos, os mesmos 12 inputs conhecidos e parse caches
separados: v6 cerca de **222 MiB de RSS**, mediana aquecida **14,7 ms**, P95
**27,5 ms**, primeira requisição **681 ms** mais inicialização **258 ms**. v5:
**812 MiB**, mediana **64,5 ms**, P95 **218 ms**, primeira requisição **3,42 s**.
Os pipelines têm comportamento/cobertura diferentes; velocidade não demonstra
melhor compreensão. Artefatos ativos v6 **38,7 MB**, contra cerca de 266 MB no
v5; a cópia ONNX float arquivada foi preservada e não carregada. Python e
dependências não entram nesse tamanho. Inferência v6 com Torch indisponível.

23 contratos passaram. A CLI executou 11 casos com memória externa e verificou
106 citações com offsets contra os inputs, sem autorização de execução.
[README](README.md) contém comandos de inferência,
reprodução, testes e refits. Só `research/frx_ai/v6/` foi acrescentado; fontes e
pesos antigos permanecem preservados. Raw data, checkpoints e relatórios com
texto ficam fora do Git. Não foram executadas suítes do aplicativo inalterado.

**Próxima hipótese:** aprender/avaliar spans e identidades canônicas de eventos
antes de polaridade, incluindo agentes humanos versus software e grupos com
coreferência/negação. Primeiro revisar os rótulos difíceis e construir outro
teste independente; não selecionar novos limiares neste teste agora conhecido.
O protótipo é útil para investigação assistida e análise de erros, não está
aprovado para interpretação ou correções autônomas.
