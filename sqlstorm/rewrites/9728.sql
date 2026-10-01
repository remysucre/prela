
WITH RankedPosts AS (
    SELECT 
        p.Id,
        p.Title,
        p.Score,
        p.ViewCount,
        p.AnswerCount,
        u.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.Id) AS rn,
        COUNT(c.Id) AS CommentCount,
        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS TagsList
    FROM 
        Posts p 
    JOIN 
        Users u ON p.OwnerUserId = u.Id 
    LEFT JOIN 
        Comments c ON p.Id = c.PostId
    LEFT JOIN 
        UNNEST(string_to_array(p.Tags, '><')) AS t(TagName) ON TRUE
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' 
    GROUP BY 
        p.Id, p.Title, p.Score, p.ViewCount, p.AnswerCount, u.DisplayName, p.Tags
), FilteredPosts AS (
    SELECT 
        rp.* 
    FROM 
        RankedPosts rp 
    WHERE 
        rp.rn = 1
)
SELECT 
    fp.Id, 
    fp.Title, 
    fp.Score, 
    fp.ViewCount, 
    fp.AnswerCount, 
    fp.CommentCount, 
    fp.OwnerDisplayName, 
    fp.TagsList
FROM 
    FilteredPosts fp
WHERE 
    fp.Score > 10 
ORDER BY 
    fp.Score DESC, 
    fp.ViewCount DESC
FETCH FIRST 100 ROWS ONLY;
