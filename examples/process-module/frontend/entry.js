export default {
  pluginApiVersion: "1.0.0",
  moduleId: "example-module",
  version: "1.2.3",
  activate(hostSdk) {
    const React = hostSdk.react;
    return {
      components: {
        ExampleOverview() {
          return React.createElement("main", null, "Example Module");
        },
      },
    };
  },
};
