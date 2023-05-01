# Technomancy Game Rules

## 0. Foundational Rules

1. All rules in Technomancy apply, up to and until a card instructs an alternative effect.
1. Rules that restrict or disallow take precedence.

## 1. Players

1. Each game is played with at least two players.
1. Before the game starts, a random player is chosen to be the starting player.
    1. The method needs to be agreed to by all players, and sufficiently random.
1. From then on, the turn order is clockwise.
    1. In a situation where it is hard to determine clockwise order, the
       players need to clear beforehand the turn order and follow it
       henceforth.
    1. This turn order is irrespective of the actual turns being taken.
1. Before the game starts, a game mode is agreed upon by all players and its
   rules applied.
1. Before the game starts, a maximum deck level is given and players may not
   play decks with a higher deck level.
    1. Any player who does play a deck with a higher deck level loses the game.

## 2. Gamemodes

1. Matrix Mode
    1. Matrix mode is played with 50 card decks.
    1. Each card in a single deck may only have a maximum of four copies.
    1. Each player starts with 20 health.
    1. Each player has a maximum hand size of 6.

## 3. Game Sequence

1. As a player starts their turn, they are assigned the current active player.
1. All other players are assigned non-active players.
1. Each turn is subdivided in the following phases:
    1. Recovery Phase
        1. All deployed cards recover.
    1. Turn Start Phase
        1. All 'At the beginning of turn' abilities trigger here.
    1. Draw Phase
        1. At the beginning of the draw phase, the active player draws one card.
        1. All 'At the beginning of the draw phase' abilities trigger here.
    1. Main Phase
        1. All 'At the beginning of the main phase' abilities trigger here.
    1. Turn End Phase
        1. All 'At the end of turn' abilities trigger here.
    1. Cleanup Phase
        1. All 'Until end of turn' effects end here.
        1. The active player discards down to hand size.

## 4. Game Actions

1. To 'recover' a card means to untap it, and all 'When ~ recovers' abilities trigger 
    1. Only deployed cards may recover.
        1. Should a recover action target an invalid target, nothing happens.
