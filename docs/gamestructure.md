The game engine execution has the following shape:

## Overview

The backend server, here denoted `Server`, is the organizing part of the whole structure. It's job is to serve as the liason between players and the engine.

```mermaid
sequenceDiagram
  Server ->>+ Engine: Create Game
  Engine ->>- Server: GameID

  Server ->>+ Engine: Start Game
  loop until Game end
    Engine ->> Engine: Step Game
    
    alt First Startup
      Engine ->> Engine: First time setup
    else Mulligan until all Players keep hand
      Engine ->> Engine: Shuffle deck and distribute hands<br>Only for non-keeping players
      Engine ->> Server: Sync state
      Engine ->>+ Server: Ask if player wants to keep
      Server ->>- Engine: List of players that are keeping
      Engine ->> Server: Sync state
    else Normal Game
      Engine ->> Engine: Determine possible actions
      Engine ->>+ Server: Let player choose from actions
      Server ->>- Engine: Reply with choice
      Engine ->> Engine: Execute choice
      Note over Server, Engine: See NormalGameLoop for details
    end
  end
  Engine ->>- Server: Report finished game state
```

## The normal game loop

Most of the time spent in a technomancy game is going to follow this sequence:


### Playing a card

```mermaid
sequenceDiagram
    participant Game
    participant ActivePlayer
    
    Game ->>+ ActivePlayer: Select card to play
    ActivePlayer ->>- Game: Selected card

    Game ->>+ ActivePlayer: Give me all choices
    ActivePlayer ->>- Game: Selected choices

    Game ->> Game: Compute Costs

    alt enough resources
        Game ->> Game: Subtract costs from AP pool
    else not enough
        Game -X Game: Cancels playing
    end
    
    Game ->> Game: Put card on stack
```
