set export
set shell := ["pwsh.exe", "-CommandWithArgs"]
set positional-arguments

move_to_copper_mine := "move -x 2 -y 0"
move_to_forge := "move -x 1 -y 5"
move_to_ash_tree := "move -x -1 -y 0"

move_to_weaponcrafting := "move -x 2 -y 1"
move_to_gearcrafting := "move -x 3 -y 1"
move_to_jewelrycrafting := "move -x 1 -y 3"
move_to_woodcuttingworkshop := "move -x -2 -y -3"

gather_copper COUNT:
    {{ move_to_copper_mine + " +x" + COUNT + " gather" }}

craft_copper_pickaxe_intro NAME gather=(move_to_copper_mine + " +x60 gather") craft_mat=(move_to_forge + " craft -code copper_bar -quantity 6") craft_item=(move_to_weaponcrafting + " craft -code copper_pickaxe"):
    cargo run -- {{ NAME + " " + gather + " " + craft_mat + " " + craft_item }}

craft_copper_pickaxe NAME equip="equip -code copper_pickaxe -slot weapon" gather=(move_to_copper_mine + " +x60 gather") craft_mat=(move_to_forge + " craft -code copper_bar -quantity 6") craft_item=(move_to_weaponcrafting + " craft -code copper_pickaxe"):
    cargo run -- {{ NAME + " " + equip + " " + gather + " " + craft_mat + " " + craft_item }}

goto NAME X Y:
    cargo run -- {{ NAME + " move -x " + X + " -y " + Y }}

goto_ash_tree NAME X="-1" Y="0":(goto NAME X Y)

craft_copper_axe NAME equip="equip -code copper_pickaxe -slot weapon" gather=(move_to_copper_mine + " +x60 gather") craft_mat=(move_to_forge + " craft -code copper_bar -quantity 6") craft_item=(move_to_weaponcrafting + " craft -code copper_axe"):
    cargo run -- {{ NAME + " " + equip + " " + gather + " " + craft_mat + " " + craft_item }}

craft_copper_dagger NAME equip="equip -code copper_pickaxe -slot weapon" gather=(move_to_copper_mine + " +x60 gather") craft_mat=(move_to_forge + " craft -code copper_bar -quantity 6") craft_item=(move_to_weaponcrafting + " craft -code copper_dagger"):
    cargo run -- {{ NAME + " " + equip + " " + gather + " " + craft_mat + " " + craft_item }}

goto_gearcrafting_workshop NAME X="3" Y="1":(goto NAME X Y)

craft_copper_helmet NAME equip="equip -code copper_pickaxe -slot weapon" gather=(move_to_copper_mine + " +x60 gather") craft_mat=(move_to_forge + " craft -code copper_bar -quantity 6") craft_item=(move_to_gearcrafting + " craft -code copper_helmet"):
    cargo run -- {{ NAME + " " + equip + " " + gather + " " + craft_mat + " " + craft_item }}

craft_copper_boots NAME equip="equip -code copper_pickaxe -slot weapon" gather=(move_to_copper_mine + " +x80 gather") craft_mat=(move_to_forge + " craft -code copper_bar -quantity 8") craft_item=(move_to_gearcrafting + " craft -code copper_boots"):
    cargo run -- {{ NAME + " " + equip + " " + gather + " " + craft_mat + " " + craft_item }}

goto_woodcutting_workshop NAME X="-2" Y="-3":(goto NAME X Y)

craft_wooden_shield NAME equip="equip -code copper_axe -slot weapon" gather=(move_to_ash_tree + " +x60 gather") craft_mat=(move_to_woodcuttingworkshop + " craft -code ash_plank -quantity 6") craft_item=(move_to_gearcrafting + " craft -code wooden_shield"):
    cargo run -- {{ NAME + " " + equip + " "+ gather + " " + craft_mat + " " + craft_item }}

craft_copper_ring NAME equip="equip -code copper_pickaxe -slot weapon" gather=(move_to_copper_mine + " +x60 gather") craft_mat=(move_to_forge + " craft -code copper_bar -quantity 6") craft_item=(move_to_jewelrycrafting + " craft -code copper_ring"):
    cargo run -- {{ NAME + " " + equip + " " + gather + " " + craft_mat + " " + craft_item }}


goto_chicken NAME X="0" Y="1":(goto NAME X Y)

goto_cooking_workshop NAME X="1" Y="1":(goto NAME X Y)

fight_chicken NAME equip="equip -code copper_dagger -slot weapon" fight=("move -x 0 -y 1 use -code cooked_chicken -quantity 1 +x4 fight"):
    cargo run -- {{ NAME + " " + equip + " " + fight }}

fight_yellow_slime NAME equip="equip -code copper_dagger -slot weapon" fight=("move -x 1 -y -2 +x2 fight rest"):
    cargo run -- {{ NAME + " " + equip + " " + fight }}

fight_sheep NAME equip="equip -code copper_dagger -slot weapon" fight=("rest move -x 5 -y 12 fight fight"):
    cargo run -- {{ NAME + " " + equip + " " + fight }}

