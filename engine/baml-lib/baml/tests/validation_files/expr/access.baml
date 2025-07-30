class Foo {
    name string
    // aliases string[]
    // pet Pet
    // color_preferences map<string, string>
}

// class Pet {
//     name string
//     age int
// }

// let foo = Foo {
//     name: "John",
//     pet: Pet {
//         name: "Buddy",
//         age: 3,
//     },
//     aliases: ["John", "Johnny"],
//     color_preferences: {
//         blue "awesome"
//         red "ok"
//         green "sweet"
//     }
// };
let foo = Foo {
    name: "John"
};

let name = foo.name;
// let pet = foo.pet;
// let alias_0 = foo.aliases[0];
// let green_preference = foo.color_preferences["green"];