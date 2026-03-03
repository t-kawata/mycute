import fs from 'fs';
import path from 'path';

function getFiles(dir) {
    let results = [];
    if (!fs.existsSync(dir)) return results;
    const list = fs.readdirSync(dir);
    list.forEach(function (file) {
        file = path.join(dir, file);
        const stat = fs.statSync(file);
        if (stat && stat.isDirectory()) {
            results = results.concat(getFiles(file));
        } else {
            results.push(file.replace(/\\/g, '/'));
        }
    });
    return results;
}

const srcFiles = getFiles('web/src');
const publicFiles = getFiles('web/public');
const allFiles = srcFiles.concat(publicFiles);

console.log(allFiles.join(' '));
