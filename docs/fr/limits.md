# Limites

Le programme généré affiche ce qu'affiche Node.js ; quelques points ne peuvent pas être identiques, et quelques-uns sont refusés. Chaque refus vient avec un message qui dit quoi, et où.

## Ce qui reste différent

- **Erreurs non attrapées.** Une erreur affichée en entier donne `Error: message`, sans la pile d'appels de JavaScript. Une erreur que rien n'attrape arrête le programme avec le code de sortie 1.
- **Les minuteries** suivent l'horloge réelle : deux minuteries à quelques millisecondes d'écart peuvent s'inverser sous forte charge, comme dans Node.js.
- **Le code construit à l'exécution** pour `eval` ou `new Function` tourne dans un petit interpréteur écrit dans le fichier, plus lent que le code traduit.
- **Les dates** sont lues et affichées en UTC.
- **Les nombres** qui restent entiers sont des `i64` : un programme qui compte sur la perte de précision des entiers au-delà de 2⁵³, comme avec les doubles de JavaScript, obtient des résultats exacts à la place.

## Ce qui est refusé

| Quoi | Pourquoi |
| --- | --- |
| `async function*` | Les générateurs asynchrones ne sont pas pris en charge. |
| `http`, `net`, `child_process` et les autres modules de Node | Seuls `fs`, `fs/promises`, `path`, `os` et `process` sont livrés. |
| Les modules natifs dans `node_modules` | Ce sont du code compilé, pas des sources à traduire. |
| Le DOM, `window`, `fetch` | Solidify traduit des programmes, pas des pages pour le navigateur. |
| `Date.UTC` | Écrivez plutôt la date sous forme de chaîne ISO. |
| Une variable qui change de type | Utilisez deux variables, ou déclarez l'union que vous voulez. |

## Rigueur

Solidify lit le TypeScript comme le fait son mode strict : lire une valeur qui peut être `null` ou `undefined` est une erreur tant qu'elle n'a pas été testée, et une variable garde le type de sa première valeur sauf si son annotation dit autre chose. [Erreurs](errors.md#les-messages-de-solidify) montre les messages et ce qu'il faut changer.

## Signaler une différence

Un programme qui se traduit mais affiche autre chose que Node.js est un bug. Signalez-le sur [GitHub](https://github.com/maxlestage/Solidify/issues) avec le programme et les deux sorties : il deviendra un programme de test une fois corrigé.
