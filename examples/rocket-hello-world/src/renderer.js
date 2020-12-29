module.exports.render = ({url, jsonData, hydrationData}) => `
  <!DOCTYPE html>
  <html>
    <head>
      <meta charset="utf-8" />
      <title>SSR</title>
    </head>
    <body>
      <div>${jsonData}</div>
    </body>
  </html>
`;
