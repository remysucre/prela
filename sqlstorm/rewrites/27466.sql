
WITH RankedPosts AS (
    SELECT
        p.Id AS PostId,
        p.Title,
        p.Body,
        p.CreationDate,
        p.ViewCount,
        p.Score,
        ARRAY_AGG(t.TagName ORDER BY t.TagName) AS TagsArray,
        COUNT(DISTINCT c.Id) AS CommentCount,
        COUNT(DISTINCT a.Id) AS AnswerCount,
        RANK() OVER (ORDER BY p.CreationDate DESC) AS PostRank
    FROM
        Posts p
    LEFT JOIN
        Comments c ON p.Id = c.PostId
    LEFT JOIN
        Posts a ON p.Id = a.ParentId
    LEFT JOIN
        LATERAL (SELECT unnest(string_to_array(SUBSTRING(p.Tags, 2, LENGTH(p.Tags) - 2), '><')) AS TagName) AS t ON TRUE
    WHERE
        p.PostTypeId = 1  
    GROUP BY
        p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.Score
),
TopPosts AS (
    SELECT
        rp.PostId,
        rp.Title,
        rp.Body,
        rp.CreationDate,
        rp.ViewCount,
        rp.Score,
        rp.TagsArray,
        rp.CommentCount,
        rp.AnswerCount,
        ROW_NUMBER() OVER (ORDER BY rp.Score DESC, rp.ViewCount DESC) AS ScoreRank
    FROM
        RankedPosts rp
    WHERE
        rp.PostRank <= 50  
)
SELECT
    tp.Title,
    tp.Body,
    tp.CreationDate,
    tp.ViewCount,
    tp.Score,
    tp.TagsArray,
    tp.CommentCount,
    tp.AnswerCount,
    u.DisplayName AS OwnerDisplayName,
    COALESCE(b.BadgeCount, 0) AS BadgeCount
FROM
    TopPosts tp
JOIN
    Users u ON u.Id = (SELECT OwnerUserId FROM Posts WHERE Id = tp.PostId)
LEFT JOIN (
    SELECT
        UserId,
        COUNT(*) AS BadgeCount
    FROM
        Badges
    WHERE
        Class = 1  
    GROUP BY
        UserId
) b ON b.UserId = u.Id
WHERE
    EXISTS (
        SELECT 1 FROM Votes v
        WHERE v.PostId = tp.PostId AND v.VoteTypeId = 2  
    )
ORDER BY
    tp.Score DESC, tp.ViewCount DESC;
