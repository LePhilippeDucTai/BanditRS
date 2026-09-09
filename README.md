<div align="center">

# BanditRS

**L'analyseur de sécurité Python [`bandit`](https://github.com/PyCQA/bandit), réécrit en Rust pur.**
Mêmes tests, mêmes options, mêmes sorties, mêmes codes de sortie — de **17× à 87× plus rapide**,
avec **57 % de mémoire en moins**.

[![Rust](https://img.shields.io/badge/rust-1.96%2B-B7410E?logo=rust&logoColor=white)](rust-toolchain.toml)
[![Licence](https://img.shields.io/badge/licence-Apache--2.0-blue)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-351%20✓%20(0%20ignor%C3%A9)-success)](#5-comment-la-parité-est-prouvée)
[![Parité](https://img.shields.io/badge/parit%C3%A9-75%2F75%20tests%20B1xx–B7xx-success)](#52-couverture-fonctionnelle--75-identifiants-sur-75)
[![Différentiel](https://img.shields.io/badge/diff%C3%A9rentiel-835%20fichiers%20%C2%B7%200%20%C3%A9cart%20inattendu-success)](#53-différentiel-contre-bandit-python)
[![Version](https://img.shields.io/badge/version-0.2.0-informational)](Cargo.toml)

</div>

---

## Sommaire

1. [En une minute](#1-en-une-minute)
2. [Comparatif des sorties en exécution](#2-comparatif-des-sorties-en-exécution)
3. [Benchmarks](#3-benchmarks)
4. [Pourquoi c'est plus rapide](#4-pourquoi-cest-plus-rapide)
5. [Comment la parité est prouvée](#5-comment-la-parité-est-prouvée)
6. [Différences résiduelles](#6-différences-résiduelles-assumées)
7. [Installation et usage](#7-installation-et-usage)
8. [Architecture](#8-architecture)
9. [Développement](#9-développement)
10. [Licence et crédits](#10-licence-et-crédits)

---

## 1. En une minute

`bandit` parcourt l'AST d'un fichier Python et exécute ~75 tests de sécurité (injection de commande,
désérialisation non sûre, crypto faible, mots de passe en dur, XML vulnérable…). BanditRS fait la même chose,
avec le parseur Python de [ruff](https://github.com/astral-sh/ruff) et une analyse parallèle par fichier.

L'objectif n'est **pas** « un linter inspiré de bandit » : c'est un **remplacement direct**. Vous pouvez
substituer le binaire dans un `pre-commit`, une CI ou un `Makefile` sans changer une seule option, et obtenir
octet pour octet la même sortie (aux exceptions documentées au [§6](#6-différences-résiduelles-assumées) près).

| | `bandit` (Python) | **BanditRS** |
|---|---|---|
| Langage / runtime | Python ≥ 3.10 + dépendances (PyYAML, stevedore, rich, GitPython…) | Rust, un binaire natif, zéro runtime |
| Identifiants de tests | 75 (42 plugins + 33 blacklists) | **les mêmes 75** |
| Options de ligne de commande longues | 27 | **les mêmes 27** |
| Formats de sortie | 9 (`csv`, `custom`, `html`, `json`, `sarif`, `screen`, `txt`, `xml`, `yaml`) | **les mêmes 9** |
| Exécutables | `bandit`, `bandit-baseline`, `bandit-config-generator` | **les mêmes 3** |
| Codes de sortie | `0` / `1` (issues) / `2` (erreur d'usage) | **identiques** |
| `# nosec`, `.bandit`, `pyproject.toml`, profils, baseline | ✅ | ✅ |
| Parallélisme | non (un fichier après l'autre) | oui (rayon, un fichier par tâche) |
| Scan de la stdlib CPython 3.11 (672 fichiers, 307 k lignes) | 17,0 s — 75 MiB | **0,20 s — 33 MiB** |

---

## 2. Comparatif des sorties en exécution

Le test décisif d'un remplacement direct n'est pas « est-ce que ça trouve les mêmes bugs », c'est
« est-ce que `diff` est vide ». Voici la réponse, sur un fichier volontairement fautif :

```python
# app.py
import subprocess
import yaml


def load(path):
    return yaml.load(open(path).read())


def run(user_input):
    subprocess.call("grep " + user_input, shell=True)


PASSWORD = "hunter2"
```

*(Sorties réelles, repliées et abrégées `…` pour tenir en deux colonnes ; le `diff` intégral suit.)*

<table>
<tr><th width="50%"><code>bandit -q app.py</code> (Python)</th><th width="50%"><code>bandit -q app.py</code> (BanditRS)</th></tr>
<tr valign="top"><td>

```
Run started:2026-09-09 09:53:52.239965+00:00

Test results:
>> Issue: [B404:blacklist] Consider possible security
   implications associated with the subprocess module.
   Severity: Low   Confidence: High
   CWE: CWE-78 (https://cwe.mitre.org/…/78.html)
   More Info: …/en/0.0.1.dev49/blacklists/…#b404-…
   Location: ./app.py:1:0
1	import subprocess
2	import yaml
3	

------------------------------------------------
>> Issue: [B506:yaml_load] Use of unsafe yaml load.
   Allows instantiation of arbitrary objects.
   Consider yaml.safe_load().
   Severity: Medium   Confidence: High
   CWE: CWE-20 (https://cwe.mitre.org/…/20.html)
   More Info: …/en/0.0.1.dev49/plugins/b506_yaml_load…
   Location: ./app.py:6:11
5	def load(path):
6	    return yaml.load(open(path).read())
7	
…
Total issues (by severity):
	Undefined: 0
	Low: 2
	Medium: 1
	High: 1
```

</td><td>

```
Run started:2026-09-09 09:53:52.273576+00:00

Test results:
>> Issue: [B404:blacklist] Consider possible security
   implications associated with the subprocess module.
   Severity: Low   Confidence: High
   CWE: CWE-78 (https://cwe.mitre.org/…/78.html)
   More Info: …/en/latest/blacklists/…#b404-…
   Location: ./app.py:1:0
1	import subprocess
2	import yaml
3	

------------------------------------------------
>> Issue: [B506:yaml_load] Use of unsafe yaml load.
   Allows instantiation of arbitrary objects.
   Consider yaml.safe_load().
   Severity: Medium   Confidence: High
   CWE: CWE-20 (https://cwe.mitre.org/…/20.html)
   More Info: …/en/latest/plugins/b506_yaml_load…
   Location: ./app.py:6:11
5	def load(path):
6	    return yaml.load(open(path).read())
7	
…
Total issues (by severity):
	Undefined: 0
	Low: 2
	Medium: 1
	High: 1
```

</td></tr>
</table>

Le `diff -u` des deux sorties complètes ne contient que **dix lignes modifiées** — cinq paires, dont quatre
fois exactement le même motif :

```diff
@@ horodatage (volatil des deux côtés) @@
-Run started:2026-09-09 09:53:52.239965+00:00
+Run started:2026-09-09 09:53:52.273576+00:00
@@ version dans l'URL de documentation (déviation #12) @@
-   More Info: …/en/0.0.1.dev49/blacklists/blacklist_imports.html#b404-import-subprocess
+   More Info: …/en/latest/blacklists/blacklist_imports.html#b404-import-subprocess
-   More Info: …/en/0.0.1.dev49/plugins/b506_yaml_load.html
+   More Info: …/en/latest/plugins/b506_yaml_load.html
-   More Info: …/en/0.0.1.dev49/plugins/b602_subprocess_popen_with_shell_equals_true.html
+   More Info: …/en/latest/plugins/b602_subprocess_popen_with_shell_equals_true.html
-   More Info: …/en/0.0.1.dev49/plugins/b105_hardcoded_password_string.html
+   More Info: …/en/latest/plugins/b105_hardcoded_password_string.html
```

Tout le reste — ordre des issues, sévérités, confiances, CWE, positions **ligne:colonne**, extraits de code,
tabulations, largeur des séparateurs, métriques finales, code de sortie — est identique octet pour octet.
Les deux seules différences sont l'horodatage (volatil par nature) et la version dans l'URL de documentation
([déviation #12](#6-différences-résiduelles-assumées), choix délibéré).

### 2.1 Les huit formats de rapport, sur tout `examples/`

Même exercice à l'échelle du corpus de fixtures complet (94 fichiers), après neutralisation des seuls champs
volatils (horodatage, version de l'outil dans l'URL de doc). Huit formats sur neuf : `screen` est le même
rendu que `txt` enrichi de séquences ANSI, il est couvert par `tests/unit_formatters_screen.rs` plutôt que
par ce comparatif.

```bash
for fmt in json txt csv xml yaml html sarif custom; do
  bandit-python -r -q -f $fmt examples | normalise > py.$fmt
  banditrs      -r -q -f $fmt examples | normalise > rs.$fmt
  diff py.$fmt rs.$fmt
done
```

| Format | Lignes de rapport | Lignes divergentes | Verdict |
|---|---:|---:|---|
| `json` | 13 293 | **0** | identique |
| `txt` | 6 073 | **0** | identique |
| `csv` | 598 | **0** | identique |
| `xml` | 1 792 | **0** | identique |
| `html` | 13 915 | **0** | identique |
| `custom` | 597 | **0** | identique |
| `sarif` | 23 973 | 4 | `"version"`/`"semanticVersion"` de l'outil lui-même (`0.0.1.dev49` vs `0.2.0`) |
| `yaml` | 11 589 | 164 | ancres/alias PyYAML (`&id001`/`*id001`) — [déviation #11](#6-différences-résiduelles-assumées) ; contenu relu identique |

Six formats sur huit sont **strictement identiques**. Les deux autres divergent uniquement dans leur
*représentation*, jamais dans les données : le SARIF annonce sa propre version (ce qui est le comportement
correct pour deux outils différents), et le YAML renonce à une optimisation de PyYAML fondée sur l'identité
d'objet Python (`id()`), que Rust n'a structurellement pas.

### 2.2 Sur du vrai code : le dépôt `bandit` lui-même

```console
$ # 69 fichiers, 8 737 lignes de code — le code source de bandit, scanné par les deux outils
$ diff <(bandit-python -r -q -f json bandit/ | normalise) \
       <(banditrs      -r -q -f json bandit/ | normalise)
$ echo $?
0
```

Rapport JSON **identique** — même unique issue trouvée, mêmes métriques par fichier, même ordre.

---

## 3. Benchmarks

### 3.1 Protocole

Aucun chiffre de cette section n'est une estimation. Tout est reproductible :

```bash
cargo build --release
scripts/bench_vs_python.sh -n 7                                       # débit corpus
scripts/bench_vs_python.sh -n 15 examples/subprocess_shell.py examples/long_set.py
```

- **Machine** : conteneur 4 vCPU (Intel Xeon @ 2,10 GHz), 15 GiB RAM, Linux 6.18.44-fc-v24.
- **Référence** : `bandit 0.0.1.dev49` (dépôt PyCQA @ `1d3053d`), CPython 3.11.15.
- **Candidat** : `banditrs 0.2.0`, profil `release` (LTO *fat*, `codegen-units = 1`).
- **Mesure** : médiane de N exécutions après une exécution de chauffe, temps mur, sortie `-f json -q`
  redirigée vers `/dev/null` (on mesure l'analyse, pas le terminal).
- **Mémoire** : `ru_maxrss` de `RUSAGE_CHILDREN`, un processus mesureur dédié par relevé.

Le protocole complet, les seuils de non-régression et le profil `callgrind` sont dans
[`docs/plan/benchmarks.md`](docs/plan/benchmarks.md).

### 3.2 Débit

| Cible | Fichiers | Lignes | `bandit` (Python) | **BanditRS** | Facteur |
|---|---:|---:|---:|---:|---:|
| `/usr/lib/python3.11` (stdlib CPython) | 672 | 307 504 | 17,02 s | **0,196 s** | **86,9×** |
| `examples/` (fixtures bandit) | 94 | 9 534 | 6,497 s | **0,341 s** | **19,0×** |
| `examples/long_set.py` (le plus gros, 65 KiB) | 1 | 7 279 | 6,247 s | **0,359 s** | **17,4×** |
| `examples/subprocess_shell.py` (fichier typique) | 1 | 60 | 0,197 s | **0,006 s** | **32,3×** |

En coût par ligne sur la stdlib : **55 µs/ligne** côté Python contre **0,64 µs/ligne** côté Rust — soit
18 kloc/s contre 1,57 Mloc/s.

> **Pourquoi `examples/` n'est « que » 19× alors que la stdlib est 87× ?**
> Parce que `examples/` est dominé par un seul fichier pathologique : `long_set.py` coûte 6,25 s des 6,50 s
> du corpus côté Python. Mesurer `examples/`, c'est donc essentiellement mesurer *un* fichier — et le
> parallélisme n'aide pas sur un fichier unique. Les 19× sont le gain **séquentiel pur** ; les 87× de la
> stdlib sont ce même gain multiplié par le parallélisme. Les deux chiffres sont cohérents, voir §3.4.

### 3.3 Latence et mémoire

| Métrique | Python | BanditRS | Écart |
|---|---:|---:|---|
| Latence sur un fichier de 60 lignes (le cas éditeur / `pre-commit`) | 197 ms | **6 ms** | −97 % |
| RSS de pointe, scan de la stdlib | 75,5 MiB | **32,7 MiB** | −57 % |
| Taille du déploiement | venv Python + ~8 dépendances | **1 binaire de 5,1 Mo** | — |

Les 6 ms de latence mono-fichier sont l'argument le plus concret au quotidien : sous 10 ms, l'analyse
devient assez rapide pour tourner **à chaque sauvegarde** dans un éditeur, ce que les ~200 ms de démarrage
de l'interpréteur Python interdisent.

### 3.4 Décomposition du gain : moteur × parallélisme

Le facteur global mélange deux effets indépendants. On les sépare en bridant rayon :

| `RAYON_NUM_THREADS` | Temps (stdlib) | Accélération vs Python | Accélération parallèle | Efficacité |
|---:|---:|---:|---:|---:|
| 1 | 0,591 s | 28,8× | 1,00× | — |
| 2 | 0,331 s | 51,4× | 1,79× | 89 % |
| 4 | 0,176 s | 96,7× | 3,36× | 84 % |

> Cette série est une campagne distincte de celle du §3.2 (médiane de 3 au lieu de 7), d'où les 0,176 s
> ici contre 0,196 s là : la variance d'exécution sur ce conteneur partagé est de l'ordre de ±10 %. Ce qui
> compte ici n'est pas la valeur absolue mais le **rapport entre les lignes**, toutes issues de la même série.

Le facteur observé se factorise donc proprement :

$$
S_{\text{total}} \;=\; \underbrace{28{,}8}_{\text{moteur séquentiel}} \;\times\; \underbrace{3{,}36}_{\text{4 cœurs}} \;\approx\; 97
$$

En injectant l'accélération à 4 cœurs dans la loi d'Amdahl, on retrouve la fraction parallélisable du
travail :

$$
S(p) = \frac{1}{(1-f) + \frac{f}{p}} \quad\Longrightarrow\quad
f = \frac{1 - 1/S(4)}{1 - 1/4} = \frac{1 - 0{,}2976}{0{,}75} \approx \mathbf{0{,}94}
$$

**94 % du travail est parallélisable** ; les 6 % restants sont le parcours du système de fichiers, la
sérialisation du rapport final et l'agrégation des métriques — tous intrinsèquement séquentiels. Le modèle
prédit donc un plafond de $1/0{,}06 \approx 16{,}7\times$ pour la seule composante parallèle : au-delà d'une
douzaine de cœurs, ajouter des cœurs ne rapportera presque plus rien, et le facteur restera gouverné par le
gain séquentiel et par l'E/S disque. Extrapolation d'un modèle à un point de mesure, à confirmer sur une
machine à plus grand nombre de cœurs.

### 3.5 Garde-fou de non-régression

Les gains ne servent à rien s'ils s'érodent silencieusement. `benches/e2e.rs` (criterion) mesure 7 points
chauds, et `scripts/bench_regression.sh` échoue si l'un d'eux se dégrade de plus de **+10 %** par rapport à
la baseline committée :

```bash
cargo bench --bench e2e            # rapport HTML dans target/criterion/
scripts/bench_regression.sh        # comparaison à la baseline, seuil +10 %
```

---

## 4. Pourquoi c'est plus rapide

Trois causes, par ordre d'importance décroissante. Aucune n'est « Rust est rapide » : ce sont des choix
d'architecture que le langage rend simplement possibles.

**1. La boucle chaude n'est plus interprétée.** `bandit` visite chaque nœud de l'AST et, pour chaque nœud,
appelle en Python une liste de fonctions-plugins récupérées dans un dictionnaire, en construisant à chaque
fois un objet `Context` enveloppant un dict. Le coût dominant n'est pas l'analyse, c'est le *dispatch* :
des centaines de millions de recherches de dict et d'appels de fonction interprétés. Côté Rust, le walker
est un `match` sur un `enum` de nœuds, les plugins sont des `fn` dans une table statique, et le `Context`
est une structure typée empruntée à l'arbre — zéro allocation par nœud.

> **Analogie.** Python fait passer chaque nœud à un standardiste qui cherche le bon interlocuteur dans un
> annuaire, décroche, transmet. Rust a câblé le standard une fois pour toutes à la compilation : chaque
> nœud arrive directement sur le bon poste.

**2. Le parseur.** `ast.parse` de CPython est écrit en C et rapide, mais il matérialise un objet Python par
nœud (avec en-tête, compteur de références, dict d'attributs). Les crates `ruff_python_parser` /
`ruff_python_ast` produisent un arbre compact d'`enum`s, sans indirection ni ramasse-miettes.

**3. Le parallélisme.** L'unité naturelle de travail de bandit est le fichier, et les fichiers sont
indépendants. Rust n'a pas de GIL : `rayon` distribue simplement un fichier par tâche. C'est ce que mesure
le facteur 3,36 du §3.4 — un facteur que la version Python ne peut pas obtenir sans passer au
multi-*processus* (et donc payer un interpréteur complet par cœur, ce qui expliquerait aussi son RSS).

**Un contre-exemple honnête.** Le profil `callgrind` ([`benchmarks.md` §6](docs/plan/benchmarks.md)) montre
que 51 % du budget d'instructions actuel part dans deux fonctions que la parité oblige à garder naïves :
`NosecLines::for_range` (30,8 %, balayage ligne à ligne des commentaires `# nosec`, quadratique en
profondeur d'imbrication) et le plugin `trojansource` (20,4 %, dix passes de recherche par ligne). Les
corriger ferait probablement gagner encore 25–30 % — mais toute optimisation qui changerait l'ordre des
issues, une position ou un texte de sortie est interdite sans entrée validée dans `DEVIATIONS.md`. **La
parité prime sur la vitesse.**

---

## 5. Comment la parité est prouvée

Se fier à un seul mode de preuve, c'est se fier à ses angles morts. BanditRS en superpose trois, qui ne
partagent aucune hypothèse : l'un lit la spécification, l'un interroge l'oracle vivant, l'un fige le passé.

> **Analogie.** Une balance neuve se vérifie de trois façons : en relisant sa notice (la suite de tests), en
> la comparant à une balance étalon (le différentiel), et en repesant chaque matin le même poids de
> référence rangé dans le tiroir (le corpus golden). La troisième est la seule qui fonctionne encore quand
> l'étalon n'est plus dans la pièce.

### 5.1 La suite de tests Python, portée test par test

Les **273 tests** de `bandit/tests/` ont servi de spécification d'acceptation. Règle du portage : *un test
Python ⇒ un test Rust homonyme, dans un fichier miroir*.

| | Nombre |
|---|---:|
| Tests Python de référence | 273 |
| → portés à l'identique | 225 |
| → adaptés (mocks Python remplacés par des fixtures réelles) | 38 |
| → non portables (introspection Python pure : `deepgetattr`, `meta_ast`…) | 10 |
| Tests Rust **sans** homologue Python (émulation d'`argparse` — cf. [§5.2](#52-couverture-fonctionnelle--75-identifiants-sur-75)) | 1 |
| **Tests Rust au total** (67 unitaires + 284 d'intégration) | **351** |
| Tests `#[ignore]` restants | **0** |

L'inventaire ligne à ligne est dans [`docs/plan/test-inventory.md`](docs/plan/test-inventory.md) ; les 10
non-portables sont justifiés un par un ([déviation #8](#6-différences-résiduelles-assumées)).

### 5.2 Couverture fonctionnelle : 75 identifiants sur 75

L'ensemble des identifiants de tests est comparé à l'énumération du chargeur d'extensions Python :

```console
$ diff <(python -c "…extension_loader.MANAGER…") <(grep -ohE 'B[0-9]{3}' src/core/registry.rs src/core/blacklist.rs | sort -u)
$ echo $?
0
```

<details>
<summary><b>Les 75 identifiants (42 plugins + 33 entrées de blacklist)</b></summary>

| Famille | Identifiants |
|---|---|
| **B1xx** — divers | `B101` `B102` `B103` `B104` `B105` `B106` `B107` `B108` `B110` `B112` `B113` |
| **B2xx** — application | `B201` `B202` |
| **B3xx** — blacklist d'appels | `B301` `B302` `B303` `B304` `B305` `B306` `B307` `B308` `B310` `B311` `B312` `B313` `B314` `B315` `B316` `B317` `B318` `B319` `B321` `B323` `B324` |
| **B4xx** — blacklist d'imports | `B401` `B402` `B403` `B404` `B405` `B406` `B407` `B408` `B409` `B411` `B412` `B413` `B415` |
| **B5xx** — crypto & certificats | `B501` `B502` `B503` `B504` `B505` `B506` `B507` `B508` `B509` |
| **B6xx** — injection | `B601` `B602` `B603` `B604` `B605` `B606` `B607` `B608` `B609` `B610` `B611` `B612` `B613` `B614` `B615` |
| **B7xx** — templating | `B701` `B702` `B703` `B704` |

</details>

De même pour la surface CLI : les **27 options longues** de `bandit --help` sont exactement celles de
BanditRS, y compris les exclusivités mutuelles (`-v`/`-q` rejeté avec le même code de sortie `2`).

L'émulation d'`argparse` par `bandit-baseline` est verrouillée octet pour octet par un test dédié — le seul
test Rust sans homologue Python (`argparse` appartient à la bibliothèque standard, la suite de bandit ne le
teste donc pas, et c'est précisément pour cela qu'un écart pouvait s'y glisser) :

| Invocation | Python | BanditRS |
|---|---|---|
| `bandit-baseline --help` / `-h` | aide sur stdout, sortie `0` | **identique** |
| `bandit-baseline` (sans cible) | `usage:` + `bandit-baseline: error: the following arguments are required: targets` sur stderr, sortie `2` | **identique** |
| `bandit-baseline -f bogus .` | `usage:` + `bandit-baseline: error: argument -f: invalid choice: 'bogus' …`, sortie `2` | **identique** |

### 5.3 Différentiel contre bandit Python

[`scripts/diff_against_python.sh`](scripts/diff_against_python.sh) exécute les deux binaires fichier par
fichier, normalise les champs volatils, et **échoue** si un fichier diverge pour une raison qui n'est pas
sur la liste blanche de `DEVIATIONS.md`.

| Corpus | Fichiers | Rapports JSON identiques | Divergences inattendues |
|---|---:|---:|---:|
| `examples/` (fixtures bandit) | 94 | **94** | **0** |
| `/usr/lib/python3.11` (stdlib CPython) | 672 | **672** | **0** |
| `bandit/` (le code source de bandit lui-même) | 69 | **69** | **0** |
| Traces de parcours AST (ordre des nœuds visités) | 91 | **91** | **0** |
| Rapports agrégés sur `examples/`, 8 formats | 8 | **7** | 1 — `yaml`, ancres PyYAML (cf. [§2.1](#21-les-huit-formats-de-rapport-sur-tout-examples)) |

```console
$ scripts/diff_against_python.sh examples
walk traces: identical=91 different=0
json identical: examples          txt identical: examples
csv identical: examples           xml identical: examples
custom identical: examples        sarif identical: examples
html identical: examples          yaml DIFF: examples (ancres PyYAML — DEVIATIONS #11)
file-by-file JSON: identical=94 whitelisted=0 unexpected=0 total=94
diff_against_python.sh: zero unexpected diff

$ scripts/diff_against_python.sh --stdlib
file-by-file JSON: identical=672 whitelisted=0 unexpected=0 total=672
diff_against_python.sh: zero unexpected diff
```

Parmi les champs que cette normalisation neutralise, un cas mérite d'être montré, parce que la divergence est
du **côté de Python** :

```diff
# examples/tarfile_extractall.py, issue B202
-"issue_text": "… members were properly validated {'Other': <ast.List object at 0x7fc651c4e170>})."
+"issue_text": "… members were properly validated {'Other': <ast.List object at 0x0>})."
```

Ce message interpole le `repr()` d'un nœud AST, lequel contient **l'adresse mémoire** de l'objet Python : la
sortie de bandit n'est donc pas reproductible d'une exécution à l'autre, ni comparable entre deux machines.
BanditRS rend `0x0` pour produire une sortie déterministe
([déviation #5](#6-différences-résiduelles-assumées)).

### 5.4 Corpus golden : la parité sans Python

Les deux modes ci-dessus supposent un interpréteur Python et un `bandit` installés. Pour que la parité reste
vérifiable *ad vitam* — en CI, sur une machine nue, dans cinq ans — les sorties de référence sont committées :

```
tests/golden/          96 fichiers, 3,7 Mo
├── examples.{json,txt,csv,xml,yaml,html,sarif,custom}   rapports agrégés sur tout examples/
├── files/                                               86 rapports JSON, un par fixture
└── normalize.rs                                         la normalisation, en un seul endroit
```

```bash
cargo test --test golden       # 9 tests, rejoue tout le corpus, aucun Python requis
```

C'est ce test qui transforme la parité d'un *constat ponctuel* en **invariant de régression** : toute
modification du moteur qui déplacerait une issue, une colonne ou un octet de rapport fait échouer la suite.

---

## 6. Différences résiduelles (assumées)

La politique est explicite : **parité stricte sur tout ce que la suite de tests observe** ; les bugs connus
de bandit ne sont corrigés que lorsque aucun test n'en dépend. Les coquilles dans les messages, l'inversion
`bad_calls`/`bad_imports` de la conversion des configurations *legacy*, l'ordre de premier match des
blacklists, le « premier hit » du `nosec` sur un `linerange`, la distinction absent ≠ `None` dans
`check_call_arg_value` : tout cela est reproduit **à l'identique**, bugs compris.

Voici les 15 écarts délibérés. Le fichier [`DEVIATIONS.md`](DEVIATIONS.md) en donne la justification
complète, cas de test à l'appui.

| # | Écart | Visible où | Nature |
|---:|---|---|---|
| 1 | `# nosec B101,B102` (virgule sans espace) : Python n'ignore que le **dernier** id, BanditRS les prend tous | fichiers avec `nosec` multi-ids | 🐞 bug corrigé |
| 2 | Formatter CSV sans CWE (`NOTSET`) : Python plante (`KeyError: 'link'`), BanditRS écrit une colonne vide | `-f csv` | 🐞 bug corrigé |
| 3 | `.bandit` (INI) : `level`/`confidence`/`number` convertis en entiers ; la clé `configfile` réellement honorée | `--ini` | 🐞 bug corrigé |
| 4 | `discover_files` ne mute plus la configuration chargée (Python accumulait les exclusions entre appels) | appels répétés | 🐞 bug corrigé |
| 5 | Adresses mémoire (`<ast.List object at 0x…>`) rendues `0x0` ; pas de `RecursionError` (walker itératif) | `B202`, fichiers très imbriqués | 🎯 déterminisme |
| 6 | Pas de barre de progression `rich` ; `--version` affiche la version du crate ; pas de ligne `running on Python x.y.z` | sortie console | 🎨 cosmétique |
| 7 | Positions des constantes de f-strings : sémantique CPython ≥ 3.12 par défaut | `-f json`, colonnes | ⚙️ `BANDITRS_PYTHON_COMPAT=3.11` |
| 8 | 10 tests Python d'introspection (`deepgetattr`, `meta_ast`…) sans équivalent Rust | suite de tests | 📐 non portable |
| 9 | `B703` / `DeepAssignation` : deux cas limites où BanditRS est **plus strict** (il ne reproduit pas deux bugs de `django_xss.py`) | fichiers ad hoc uniquement | 🐞 bug corrigé |
| 10 | YAML : les scalaires *double-quoted* (non-ASCII) ne sont pas repliés à 80 colonnes | `-f yaml`, rare | 🎨 cosmétique |
| 11 | YAML : pas d'ancres/alias `&id001`/`*id001` (PyYAML déduplique par `id()` d'objet Python) | `-f yaml` | 🎨 cosmétique |
| 12 | `more_info` pointe `…/en/latest/…` au lieu de la version installée du paquet | tous formats | 🎯 choix délibéré |
| 13 | `Context::statement()` a une vraie implémentation (en Python la clé n'est jamais écrite, la propriété vaut toujours `None`) | aucun plugin ne la lit | 📐 testabilité |
| 14 | Le log « Using command line arg for selected targets » n'est émis que si la clé `targets` est dans le `.bandit` | `-v --ini` | 🎨 cosmétique |
| 15 | `bandit-baseline` : « Got current commit: `master` » au lieu de « `<sha> master` » | `bandit-baseline` | 🎨 cosmétique |

**Aucun de ces écarts ne change quelle issue est remontée, à quelle ligne, avec quelle sévérité** — à
l'exception assumée du #9 (deux cas limites de `B703` qu'aucune fixture upstream n'exerce) et du #1 (une
syntaxe `nosec` que Python traite manifestement de travers).

---

## 7. Installation et usage

### Compilation

```bash
rustup update stable          # rustc ≥ 1.96 requis
git clone https://github.com/LePhilippeDucTai/BanditRS && cd BanditRS
cargo build --release
# → target/release/{bandit, bandit-baseline, bandit-config-generator}
```

### Usage

Toutes les invocations de `bandit` fonctionnent telles quelles :

```bash
bandit -r mon_projet/                          # scan récursif
bandit -r mon_projet/ -f json -o report.json   # rapport JSON
bandit -r mon_projet/ -ll -ii                  # sévérité ≥ MEDIUM, confiance ≥ MEDIUM
bandit -r mon_projet/ -s B101,B601             # ignorer des tests
bandit -r mon_projet/ -t B602                  # ne lancer qu'un test
bandit -r mon_projet/ -c bandit.yaml           # profil de configuration
bandit -r mon_projet/ -b baseline.json         # ne montrer que les issues nouvelles
bandit -r mon_projet/ --exit-zero              # ne jamais échouer (mode rapport)
bandit --help                                  # les 27 options
```

Les deux outils annexes aussi :

```bash
bandit-config-generator --show-defaults > bandit.yaml   # générer un profil complet
bandit-baseline -r mon_projet/                          # diff vs le commit parent (requiert git)
```

### Remplacer bandit dans une CI

```yaml
# .pre-commit-config.yaml — avant
- repo: https://github.com/PyCQA/bandit
  rev: <tag>
  hooks: [{ id: bandit, args: ["-r", "src/"] }]

# après : même invocation, même sortie, même code de sortie
- repo: local
  hooks:
    - id: banditrs
      name: banditrs
      entry: /chemin/vers/bandit
      language: system
      types: [python]
      args: ["-r", "src/"]
```

### Outil de développement

```bash
bandit --dump-walk examples/nosec.py     # trace du parcours AST (ordre des nœuds, contextes)
BANDITRS_PYTHON_COMPAT=3.11 bandit …     # positions de f-strings à la CPython 3.11
```

---

## 8. Architecture

Ce n'est pas une transcription ligne à ligne : c'est un redesign idiomatique (pas de dictionnaires
dynamiques, pas d'état global mutable, parallélisme par fichier) sous contrainte de sortie identique.

```
src/
├── ast/         2 586 l.  parcours de l'AST ruff : walker, linerange, qualname, positions,
│                          littéraux, f-strings, trace de débogage
├── core/        4 654 l.  moteur : config, profils, registre de plugins, blacklists, tester,
│                          contexte, nosec, découverte de fichiers, métriques, scan parallèle
├── plugins/     2 399 l.  les 42 plugins (32 fichiers), messages et seuils verbatim
├── formatters/  1 464 l.  les 9 formats de sortie
├── cli/         1 579 l.  argparse compatible, bandit-baseline, bandit-config-generator
├── pycompat/    2 587 l.  émulation exacte de la bibliothèque standard Python là où la sortie
│                          en dépend : PyYAML (repliage à 80 colonnes), configparser, format(),
│                          repr() des littéraux, encodages
└── source/        253 l.  lecture/décodage des fichiers, index de lignes, extraits
```

| | Python (`bandit`) | Rust (BanditRS) |
|---|---:|---:|
| Code source | 11 234 lignes | 15 961 lignes |
| Tests | 5 053 lignes | 5 725 lignes |

Le module `pycompat/` (2 587 lignes, 16 % du code) mérite un mot : il n'implémente aucune analyse de
sécurité. Il existe uniquement parce que la sortie de bandit expose des détails d'implémentation de la
bibliothèque standard Python — l'algorithme de repliage de lignes de PyYAML, la syntaxe de `str.format()`,
le `repr()` des littéraux, la sémantique de `configparser`. **C'est le prix réel d'une parité octet pour
octet**, et il est plus élevé que celui du moteur d'analyse lui-même.

### Dépendances

`ruff_python_{parser,ast}` · `ruff_{text_size,source_file}` (parseur) · `rayon` (parallélisme) · `regex` ·
`rustc-hash` · `indexmap` · `serde`/`serde_json` · `toml` · `saphyr-parser` · `encoding_rs` · `typed-arena`.

---

## 9. Développement

### Porte de qualité

```bash
scripts/check.sh          # build + 351 tests + rustfmt + clippy -D warnings + compilation des benchs
scripts/check.sh fast     # sans les benchs (boucle de développement)
```

`scripts/check.sh` reproduit à l'identique les trois jobs du workflow GitHub Actions, conservé mais inerte
dans `.github/workflows/ci.yml.disabled`. Sortie `0` = équivalent d'une CI verte.

### Vérifier la parité soi-même

```bash
cargo test --test golden                      # rejoue le corpus golden — aucun Python requis
scripts/diff_against_python.sh examples       # différentiel sur les fixtures
scripts/diff_against_python.sh --stdlib       # différentiel sur 672 fichiers de la stdlib
scripts/gen_golden.sh                         # régénérer le corpus golden
```

Le différentiel attend un `bandit` Python de référence ; par défaut
`/home/user/.pyenv-bandit/bin/bandit`, redéfinissable par la variable `PY_BANDIT`.

### Documentation interne

| Fichier | Contenu |
|---|---|
| [`PLAN.md`](PLAN.md) | état d'avancement, décisions, jalons, architecture, pièges rencontrés |
| [`DEVIATIONS.md`](DEVIATIONS.md) | les 15 écarts délibérés, justifiés un par un |
| [`docs/spec/core.md`](docs/spec/core.md) | sémantique exacte du cœur Python (à lire avant tout portage) |
| [`docs/spec/plugins.md`](docs/spec/plugins.md) | les 42 plugins + blacklists : messages, regex, défauts verbatim |
| [`docs/spec/cli_formatters_tests.md`](docs/spec/cli_formatters_tests.md) | CLI, formatters, suite de tests comme spécification |
| [`docs/plan/benchmarks.md`](docs/plan/benchmarks.md) | protocole de mesure, seuils, résultats, profil `callgrind` |
| [`docs/plan/test-inventory.md`](docs/plan/test-inventory.md) | correspondance test Python ↔ test Rust, ligne à ligne |
| [`docs/plan/README.md`](docs/plan/README.md) | le plan de développement parallèle (16 lots), conservé en historique |

---

## 10. Licence et crédits

Apache-2.0 — voir [`LICENSE`](LICENSE) et [`NOTICE`](NOTICE).

BanditRS est une réimplémentation indépendante de [**bandit**](https://github.com/PyCQA/bandit) (PyCQA), dont
il reprend l'intégralité de la sémantique, les messages, les fixtures de `examples/` et les spécifications de
tests. Tout le mérite de la conception des règles de sécurité revient au projet original.

Le parseur Python vient de [**ruff**](https://github.com/astral-sh/ruff) (Astral).
