WITH RankedPosts AS (
    SELECT
        p.Id AS PostId,
        p.Title,
        p.Body,
        p.Tags,
        p.CreationDate,
        p.ViewCount,
        p.Score,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
        ROW_NUMBER() OVER (PARTITION BY p.Tags ORDER BY p.Score DESC, p.ViewCount DESC, p.Id) AS Rank
    FROM
        Posts p
    LEFT JOIN
        Votes v ON p.Id = v.PostId
    WHERE
        p.PostTypeId = 1 
    GROUP BY
        p.Id, p.Title, p.Body, p.Tags, p.CreationDate, p.ViewCount, p.Score
),
TopPosts AS (
    SELECT
        rp.PostId,
        rp.Title,
        rp.Body,
        rp.Tags,
        rp.Rank
    FROM
        RankedPosts rp
    WHERE
        rp.Rank <= 5 
),
TagStatistics AS (
    SELECT
        SPLIT_PART(tags, '><', 1) AS TagName,
        COUNT(*) AS PostCount,
        AVG(ViewCount) AS AverageViews
    FROM
        Posts
    WHERE
        PostTypeId = 1 
    GROUP BY
        TagName
)
SELECT
    tp.PostId,
    tp.Title,
    tp.Body,
    tp.Tags,
    ts.TagName,
    ts.PostCount,
    ts.AverageViews
FROM
    TopPosts tp
JOIN
    TagStatistics ts ON tp.Tags LIKE '%' || ts.TagName || '%'
ORDER BY
    ts.PostCount DESC, ts.AverageViews DESC;