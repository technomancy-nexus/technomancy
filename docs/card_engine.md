Card Engine Usage
=================


Scripting the card engine is done through a detailed process of individual
rules that get applied to the gamestate.

The central idea is that the game advances step by step, each new step moving the game forward.
It is the combination of invidiual steps that then composes the game.

All card games that can be represented by the card engine follow this outline:

- The GameState starts out _empty_
- Global game rules are added, these can potentially be sizeable for a high complexity game
    - Rules are pieces of behaviour that dictate the game
    - For example "Every time a player draws a card where the property 'global
      action' is defined, do ..." could be a valid rule
    - Rules get 'triggered' depending on their condition:
        - GameSetup
        - A card is added to a zone
        - A property of a card changes
        - TurnStart
        - etc...


Then later:
- Global Zones are added (think global discard/draw piles)
- Players get added & initialized 
- Any extra game initialization is done (potentially depending on the players, like mulligans etc...)

