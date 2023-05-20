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
1. On each phase the active player gets priority
    1. Whenever a player chooses not to play a card or activate an ability, they pass priority
    1. The next player to receive priority is determined by turn order (usually clockwise)
    1. Whenever a player receives priority, the game first does the following actions:
        1. Check for all state-based effects, repeat until no effect is applied
        1. Any triggered effects due to the state based effects go on the stack
        1. The game repeats until no effect is applied nor any triggered effect is triggered
    1. Once all players have chosen to pass priority, and the stack is empty
       the current phase ends and the next phase starts
    1. Once all players have chosen to pass priorty, and the stack is not
       empty, the top most card or ability takes its effect
    1. Once a card or ability has taken effect, the active player receives priority
    1. Players do not receive priority during the recovery phase
    1. Players do not receive priority in the cleanup phase unless a triggered
       ability goes on the stack during it
1. There exist the following type of cards
    1. Quickhack
        1. Quickhacks may be played at anytime that a player has priority
        1. After being assembled a Quickhack goes into the discard pile.
    1. Program
        1. Programs may be played only when the player is the active player and the stack is empty.
        1. After being assembled a Program goes into the discard pile.
    1. Agent
        1. Agents may be played only when the player is the active player and the stack is empty.
        1. After being assembled an Agent goes onto the battlefield.
    1. Building
        1. Buildings may be played only when the player is the active player and the stack is empty.
        1. Only one building may be played per turn.
        1. Buildings do not go on the stack and instead directly enter the battlefield.
1. Whenever the player with priority plays a card, they perform the following actions:
    1. They declare which card they wish to play
    1. They determine the scrip cost of the card
        1. They add any additional cost for playing this card
        1. They substract any potential costs for playing this card
        1. They may then activate any scrip abilities of battlefield cards you control
        1. This determines the final cost of the card
    1. The user then pays those costs. If for some reason they can't the
       playing is aborted and the game goes back to before the casting.
    1. Once paid, the card goes to the stack

## 4. Game Actions

1. To 'recover' a card means to untap it, and all 'When ~ recovers' abilities trigger 
    1. Only deployed cards may recover.
        1. Should a recover action target an invalid target, nothing happens.

## 5. Game Structure

1. Every card has a singular unique name
1. A cards owner is the player whose library it started in at the beginning of the game.
    1. A card created throughout the game is owned by the player that created it.
1. Every card has none or one or more factions, which are determined by the
   different kinds of scrip that appear in its cost.
    1. A card without a faction is called 'factionless'
1. Every card has a scrip cost
    1. A card's 'scrip cost' is the sum of all its kinds of scrips that exist in its cost
1. There are five different kinds of scrip, each issued by the respective megacorp
    1. CORP1
    1. CORP2
    1. CORP3
    1. CORP4
    1. CORP5
1. Agents have two properties: Damage and Health
    1. An agent whose health is 0 dies and is put into the discard pile
    1. Whenever an agent is dealt damage they lose that much health
    1. An agents maximum health is the health as written on the card

## 6. Resolving Effects

1. Every card has certain properties. These properties have a unique timestamp
   of when they are first applied.
1. Resolving in which order these properties apply is done with the following steps:
    1. Sort all properties by their timestamp in ascending order. Later timestamps are after earlier.
    1. Consider all card properties.
    1. Consider all control effects.
    1. Consider all alliegence effects.
    1. Consider all power and health changing properties.
    1. Consider all other effects.
