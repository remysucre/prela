WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Body,
        p.CreationDate,
        p.ViewCount,
        p.Score,
        p.Tags,
        p.OwnerUserId,
        u.DisplayName AS OwnerDisplayName,
        RANK() OVER (PARTITION BY p.OwnerUserId ORDER BY p.Score DESC, p.CreationDate DESC) AS PostRank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.PostTypeId = 1 
),
TagStatistics AS (
    SELECT 
        tag,
        COUNT(*) AS PostCount,
        AVG(ViewCount) AS AvgViewCount,
        AVG(Score) AS AvgScore
    FROM (
        SELECT 
            unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS tag,
            p.ViewCount,
            p.Score
        FROM 
            Posts p
        WHERE 
            p.PostTypeId = 1 
    ) AS TagData
    GROUP BY 
        tag
),
TopAuthors AS (
    SELECT 
        u.Id AS UserId,
        u.DisplayName,
        COUNT(p.Id) AS PostCount,
        SUM(p.Score) AS TotalScore
    FROM 
        Users u
    JOIN 
        Posts p ON u.Id = p.OwnerUserId
    WHERE 
        p.PostTypeId = 1 
    GROUP BY 
        u.Id, u.DisplayName
    HAVING 
        COUNT(p.Id) > 5 
),
FinalBenchmark AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.OwnerDisplayName,
        rp.CreationDate,
        ts.tag,
        ts.PostCount,
        ts.AvgViewCount,
        ts.AvgScore,
        ta.UserId,
        ta.DisplayName AS TopAuthorName,
        ta.PostCount AS TopAuthorPostCount,
        ta.TotalScore AS TopAuthorTotalScore
    FROM 
        RankedPosts rp
    JOIN 
        TagStatistics ts ON POSITION(ts.tag IN rp.Tags) > 0
    JOIN 
        TopAuthors ta ON rp.OwnerUserId = ta.UserId
    WHERE 
        rp.PostRank = 1 
)
SELECT 
    PostId,
    Title,
    OwnerDisplayName,
    CreationDate,
    tag,
    PostCount,
    AvgViewCount,
    AvgScore,
    UserId AS TopAuthorId,
    TopAuthorName,
    TopAuthorPostCount,
    TopAuthorTotalScore
FROM 
    FinalBenchmark
ORDER BY 
    AvgScore DESC, AvgViewCount DESC, PostId, tag
LIMIT 50;