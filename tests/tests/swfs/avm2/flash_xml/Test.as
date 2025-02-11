package {
  import flash.display.Sprite;
  public class Test extends Sprite { }
}

import flash.xml.XMLDocument;

var doc = new XMLDocument('<foo>bar</foo><x>y</x>');
trace("doc.nodeType: " + doc.nodeType);
trace("doc.nodeName: " + doc.nodeName);
trace("doc.childNodes: " + doc.childNodes);
trace("doc.toString(): " + doc.toString());
trace("doc.firstChild.nodeType: " + doc.firstChild.nodeType);
trace("doc.firstChild.nodeName: " + doc.firstChild.nodeName);
trace("doc.firstChild.toString(): " + doc.firstChild.toString());
trace("doc.firstChild.firstChild.nodeType: " + doc.firstChild.firstChild.nodeType);
trace("doc.firstChild.firstChild.nodeName: " +  doc.firstChild.firstChild.nodeName);
trace("doc.firstChild.firstChild.toString(): " + doc.firstChild.firstChild.toString());
trace("doc.firstChild.nextSibling: " + doc.firstChild.nextSibling);
trace("doc.firstChild.nextSibling.nodeName: " + doc.firstChild.nextSibling.nodeName);

trace("///")

var doc2 = new XMLDocument();
doc2.parseXML('<a hello="world">xxx</a><B>yyy</B>');
trace("doc2.nodeType: " + doc2.nodeType);
trace("doc2.nodeName: " + doc2.nodeName);
trace("doc2.childNodes: " + doc2.childNodes);
trace("doc2.toString(): " + doc2.toString());
trace("doc2.firstChild.nodeName: " + doc2.firstChild.nodeName);
trace("doc2.firstChild.toString(): " + doc2.firstChild.toString());
trace("doc2.firstChild.nextSibling: " + doc2.firstChild.nextSibling);
trace("JSON.stringify(doc2.firstChild.attributes): " + JSON.stringify(doc2.firstChild.attributes));
trace("doc2.firstChild.nextSibling.nodeName: " + doc2.firstChild.nextSibling.nodeName);
